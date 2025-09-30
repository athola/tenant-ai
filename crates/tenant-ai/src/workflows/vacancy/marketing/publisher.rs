use chrono::NaiveDate;

use std::fmt::Write as _;

use super::drive::{DriveGateway, DriveMedia, DriveOperationError};
use crate::workflows::vacancy::applications::compliance::ComplianceGuard;
use crate::workflows::vacancy::applications::evaluation::EvaluationEngine;
use crate::workflows::vacancy::applications::{
    self, ApplicantProfile, ApplicationDecision, ApplicationId, ApplicationSubmission,
    EvaluationConfig,
};

#[derive(Debug, Clone)]
pub struct ListingContext {
    pub unit_id: String,
    pub property_code: String,
    pub property_name: String,
    pub address: String,
    pub bedrooms: u8,
    pub bathrooms: f32,
    pub square_feet: u16,
    pub rent: u32,
    pub deposit: u32,
    pub amenities: Vec<String>,
    pub neighborhood_highlights: Vec<String>,
    pub nearby_schools: Vec<String>,
    pub drive_folder_id: String,
    pub available_on: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct ProspectOutcome {
    pub name: String,
    pub decision: String,
    pub rationale: String,
}

#[derive(Debug, Clone)]
pub struct ProspectCandidate {
    pub name: String,
    pub submission: ApplicationSubmission,
}

#[derive(Debug, Clone)]
pub struct MarketingInput {
    pub listing: ListingContext,
    pub sample_applicants: Vec<ProspectCandidate>,
}

#[derive(Debug, Clone)]
pub struct MarketingPlan {
    pub description: String,
    pub google_doc_id: String,
    pub selected_photos: Vec<DriveMedia>,
    pub missing_photos: bool,
    pub compliance_summary: String,
    pub prospect_outcomes: Vec<ProspectOutcome>,
}

#[derive(Debug, thiserror::Error)]
pub enum MarketingError {
    #[error(transparent)]
    Drive(#[from] DriveOperationError),
    #[error("unable to evaluate applicant: {0}")]
    Evaluation(String),
}

#[derive(Debug)]
pub struct MarketingPublisher {
    drive: Box<dyn DriveGateway>,
    config: EvaluationConfig,
}

impl MarketingPublisher {
    pub fn new(drive: Box<dyn DriveGateway>, config: EvaluationConfig) -> Self {
        Self { drive, config }
    }

    pub fn prepare_listing(&self, input: MarketingInput) -> Result<MarketingPlan, MarketingError> {
        let MarketingInput {
            listing,
            sample_applicants,
        } = input;

        let media = self.drive.list_unit_media(&listing.drive_folder_id)?;
        let selected_photos: Vec<DriveMedia> = media
            .into_iter()
            .filter(|item| match item.mime_type.as_deref() {
                Some(mime) => mime.starts_with("image/"),
                None => true,
            })
            .collect();
        let missing_photos = selected_photos.is_empty();

        let description = build_listing_description(&listing, !missing_photos, &self.config);
        let compliance_summary = build_compliance_summary(&self.config);

        let html_body = render_listing_html(
            &listing,
            &description,
            &selected_photos,
            &compliance_summary,
        );
        let doc_title = format!(
            "{} {} Listing Marketing Draft",
            listing.property_name, listing.unit_id
        );
        let google_doc_id = self.drive.create_listing_document(
            &doc_title,
            &html_body,
            Some(listing.drive_folder_id.as_str()),
        )?;

        let outcomes = self.evaluate_sample_applicants(sample_applicants)?;

        Ok(MarketingPlan {
            description,
            google_doc_id,
            selected_photos,
            missing_photos,
            compliance_summary,
            prospect_outcomes: outcomes,
        })
    }
}

impl MarketingPublisher {
    fn evaluate_sample_applicants(
        &self,
        applicants: Vec<ProspectCandidate>,
    ) -> Result<Vec<ProspectOutcome>, MarketingError> {
        if applicants.is_empty() {
            return Ok(Vec::new());
        }

        let guard = ComplianceGuard::from_config(&self.config);
        let engine = EvaluationEngine::new(self.config.clone());

        applicants
            .into_iter()
            .map(|candidate| {
                let mut profile = guard
                    .profile_from_submission(candidate.submission)
                    .map_err(|err| {
                        MarketingError::Evaluation(format!("{}: {}", candidate.name, err))
                    })?;

                profile.application_id = ApplicationId(slugify_demo_id(&candidate.name));
                let outcome = engine.score(&profile);
                Ok(ProspectOutcome {
                    name: candidate.name,
                    decision: outcome.decision.summary(),
                    rationale: format_outcome_rationale(&outcome, &profile),
                })
            })
            .collect()
    }
}

fn build_listing_description(
    listing: &ListingContext,
    has_media: bool,
    config: &EvaluationConfig,
) -> String {
    let mut content = String::new();
    let available_str = listing.available_on.format("%B %d, %Y").to_string();
    let minimum_income_multiplier = if config.minimum_rent_to_income_ratio > 0.0 {
        1.0 / f64::from(config.minimum_rent_to_income_ratio)
    } else {
        0.0
    };

    let income_requirement =
        if minimum_income_multiplier.is_finite() && minimum_income_multiplier > 0.0 {
            format!(
                "steady verifiable income ≥ {:.1}x rent",
                minimum_income_multiplier
            )
        } else {
            "steady verifiable income meeting published criteria".to_string()
        };

    writeln!(
        &mut content,
        "{} {} — Available {}",
        listing.property_name, listing.unit_id, available_str
    )
    .expect("write headline");
    writeln!(&mut content, "Address: {}", listing.address).expect("write address");
    writeln!(
        &mut content,
        "{} bedroom / {:.1} bath | {} sq ft | ${} per month",
        listing.bedrooms, listing.bathrooms, listing.square_feet, listing.rent
    )
    .expect("write specs");
    content.push('\n');

    if !listing.amenities.is_empty() {
        writeln!(&mut content, "Amenities: {}", listing.amenities.join(", "))
            .expect("write amenities");
    }

    if !listing.neighborhood_highlights.is_empty() {
        writeln!(
            &mut content,
            "Neighborhood highlights: {}",
            listing.neighborhood_highlights.join(", ")
        )
        .expect("write neighborhood");
    }

    if !listing.nearby_schools.is_empty() {
        writeln!(
            &mut content,
            "Nearby schools: {}",
            listing.nearby_schools.join(", ")
        )
        .expect("write schools");
    }

    if has_media {
        writeln!(
            &mut content,
            "Marketing assets: Refreshed photo set pulled from Google Drive listing archive."
        )
        .expect("write media note");
    } else {
        writeln!(
            &mut content,
            "Marketing assets: Requesting refreshed photography to keep the listing current."
        )
        .expect("write missing media note");
    }

    content.push('\n');
    writeln!(
        &mut content,
        "Prequalifiers: No smoking, no pets (Service animals always welcome), {income_requirement}, no violent criminal history within the past seven years, and applicants must not be on any sex offender registry.",
    )
    .expect("write prequalifiers");
    writeln!(
        &mut content,
        "We proudly comply with the Fair Housing Act and the Iowa Civil Rights Act. Marketing language focuses on unit features and availability without steering or excluding protected classes."
    )
    .expect("write compliance");

    content
}

fn build_compliance_summary(config: &EvaluationConfig) -> String {
    format!(
        "Compliance guard rails: Fair Housing Act & Iowa Civil Rights Act honored; deposit capped at {:.1}x rent; violent felonies screened within {} years; smoking and pet policies applied uniformly with service animals accommodated.",
        config.deposit_cap_multiplier,
        config.violent_felony_lookback_years
    )
}

fn render_listing_html(
    listing: &ListingContext,
    description: &str,
    photos: &[DriveMedia],
    compliance_summary: &str,
) -> String {
    let mut html = String::new();
    writeln!(
        html,
        "<h1>{} {} — Available {}</h1>",
        listing.property_name,
        listing.unit_id,
        listing.available_on.format("%B %d, %Y")
    )
    .expect("write heading");
    writeln!(html, "<p>{}</p>", escape_html(&listing.address)).expect("address paragraph");

    for paragraph in description
        .split('\n')
        .filter(|line| !line.trim().is_empty())
    {
        writeln!(html, "<p>{}</p>", escape_html(paragraph.trim())).expect("description paragraphs");
    }

    if !photos.is_empty() {
        html.push_str("<h2>Selected Media</h2><ul>");
        for photo in photos {
            let label = escape_html(&photo.name);
            if let Some(link) = &photo.web_view_link {
                writeln!(
                    html,
                    "<li><a href=\"{}\">{}</a></li>",
                    escape_html(link),
                    label
                )
                .expect("photo link");
            } else {
                writeln!(html, "<li>{}</li>", label).expect("photo item");
            }
        }
        html.push_str("</ul>");
    }

    writeln!(html, "<p><em>{}</em></p>", escape_html(compliance_summary))
        .expect("compliance paragraph");

    html
}

fn format_outcome_rationale(
    outcome: &applications::EvaluationOutcome,
    profile: &ApplicantProfile,
) -> String {
    let core = match &outcome.decision {
        ApplicationDecision::Approved => format!(
            "Approved with composite score {}; applicant meets published lawful factors.",
            outcome.total_score
        ),
        ApplicationDecision::ConditionalApproval { required_actions } => {
            if required_actions.is_empty() {
                format!(
                    "Conditional approval with composite score {}; lawful follow-up required.",
                    outcome.total_score
                )
            } else {
                format!(
                    "Conditional approval (score {}): {}.",
                    outcome.total_score,
                    required_actions.join(", ")
                )
            }
        }
        ApplicationDecision::Denied(reason) => format!(
            "Denied (score {}): {}.",
            outcome.total_score,
            reason.summary()
        ),
        ApplicationDecision::ManualReview { reasons } => {
            if reasons.is_empty() {
                format!("Manual review required (score {}).", outcome.total_score)
            } else {
                format!(
                    "Manual review required (score {}): {}.",
                    outcome.total_score,
                    reasons.join("; ")
                )
            }
        }
    };

    let mut additional = String::new();
    if !profile.criminal_history.is_empty() {
        additional.push_str(
            " Criminal background reviewed in accordance with HUD disparate impact guidance.",
        );
    }

    format!(
        "{} Decision is communicated with Fair Housing-compliant adverse action language when necessary.{}",
        core,
        additional
    )
}

fn escape_html(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn slugify_demo_id(name: &str) -> String {
    let mut slug = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "demo-applicant".to_string()
    } else {
        format!("demo-{}", trimmed)
    }
}
