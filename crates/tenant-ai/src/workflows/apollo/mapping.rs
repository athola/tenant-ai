use super::normalizer::normalize_name;
use std::collections::HashMap;
use std::sync::OnceLock;

static APOLLO_NAME_MAP: OnceLock<HashMap<String, &'static str>> = OnceLock::new();

pub(crate) fn task_key_for_normalized(normalized_name: &str) -> Option<&'static str> {
    apollo_name_map().get(normalized_name).copied()
}

fn apollo_name_map() -> &'static HashMap<String, &'static str> {
    APOLLO_NAME_MAP.get_or_init(|| {
        const NAME_TO_TASK: &[(&str, &str)] = &[
            // Marketing & Advertising
            ("Create and Publish Listing - Leasing Agent", "marketing_publish_listing"),
            ("Create and Publish Listing \u{2013} Leasing Agent", "marketing_publish_listing"),
            ("Create and Publish Listing", "marketing_publish_listing"),
            ("Update Vacancy in AppFolio - Leasing Agent", "marketing_update_appfolio"),
            ("Update Vacancy in AppFolio \u{2013} Leasing Agent", "marketing_update_appfolio"),
            ("Update Vacancy in AppFolio", "marketing_update_appfolio"),
            // Screening & Application
            (
                "Manage Inquiries and Schedule Showings - Leasing Agent",
                "screening_manage_inquiries",
            ),
            (
                "Manage Inquiries and Schedule Showings \u{2013} Leasing Agent",
                "screening_manage_inquiries",
            ),
            (
                "Manage Inquiries & Schedule Showings - Leasing Agent",
                "screening_manage_inquiries",
            ),
            (
                "Manage Inquiries and Schedule Showings",
                "screening_manage_inquiries",
            ),
            (
                "Process Rental Applications - Leasing Agent",
                "screening_process_applications",
            ),
            (
                "Process Rental Applications \u{2013} Leasing Agent",
                "screening_process_applications",
            ),
            (
                "Process Rental Applications",
                "screening_process_applications",
            ),
            (
                "Notify Applicants of Status - Leasing Agent",
                "screening_notify_applicants",
            ),
            (
                "Notify Applicants of Status \u{2013} Leasing Agent",
                "screening_notify_applicants",
            ),
            (
                "Notify Applicants of Status",
                "screening_notify_applicants",
            ),
            // Lease Signing & Move-In
            (
                "Prepare Lease Agreement - Leasing Agent",
                "leasing_prepare_agreement",
            ),
            (
                "Prepare Lease Agreement \u{2013} Leasing Agent",
                "leasing_prepare_agreement",
            ),
            ("Prepare Lease Agreement", "leasing_prepare_agreement"),
            (
                "Complete Lease Agreement and Collect Financials - Leasing Agent",
                "leasing_prepare_agreement",
            ),
            (
                "Complete Lease Agreement and Collect Financials",
                "leasing_prepare_agreement",
            ),
            (
                "Send the lease to the new tenant for e-signature via AppFolio.",
                "leasing_prepare_agreement",
            ),
            (
                "Send the new lease agreement to the tenant for signature. Iowa law (Iowa Code \u{00a7} 562A.13) requires written notice of any rent increase at least 30 days before the effective date.",
                "leasing_prepare_agreement",
            ),
            ("Sign new leases", "leasing_prepare_agreement"),
            (
                "Collect Funds - Property Manager/Accounting",
                "leasing_collect_funds",
            ),
            (
                "Collect Funds \u{2013} Property Manager/Accounting",
                "leasing_collect_funds",
            ),
            (
                "Collect Funds - Property Manager / Accounting",
                "leasing_collect_funds",
            ),
            (
                "Collect Funds - Property Manager & Accounting",
                "leasing_collect_funds",
            ),
            (
                "Collect Funds - PM/Accounting",
                "leasing_collect_funds",
            ),
            ("Collect Funds", "leasing_collect_funds"),
            (
                "Collect Move-In Funds - Property Manager/Accounting",
                "leasing_collect_funds",
            ),
            ("Collect Move-In Funds", "leasing_collect_funds"),
            (
                "Collect first month's rent and the security deposit.",
                "leasing_collect_funds",
            ),
            (
                "Conduct Move-In Inspection - Property Manager",
                "leasing_conduct_move_in_inspection",
            ),
            (
                "Conduct Move-In Inspection \u{2013} Property Manager",
                "leasing_conduct_move_in_inspection",
            ),
            (
                "Conduct Move-In Inspection",
                "leasing_conduct_move_in_inspection",
            ),
            (
                "Conduct Move-In Walk-Through & Orientation - Property Manager",
                "leasing_conduct_move_in_inspection",
            ),
            (
                "Conduct Move-In Walk-Through & Orientation \u{2013} Property Manager",
                "leasing_conduct_move_in_inspection",
            ),
            (
                "Conduct Move-In Walk-Through and Orientation - Property Manager",
                "leasing_conduct_move_in_inspection",
            ),
            (
                "Complete LIHTC Initial Certification - Compliance Coordinator",
                "leasing_lihtc_certification",
            ),
            (
                "Complete LIHTC Initial Certification \u{2013} Compliance Coordinator",
                "leasing_lihtc_certification",
            ),
            (
                "Complete LIHTC Initial Certification",
                "leasing_lihtc_certification",
            ),
            ("Finalize TIC", "leasing_lihtc_certification"),
            // Handoff
            (
                "Start New Resident Workflow",
                "handoff_start_new_resident_workflow",
            ),
            (
                "Start the New Resident Workflow",
                "handoff_start_new_resident_workflow",
            ),
            (
                "Hand Over Keys & Welcome Tenant - Leasing Agent",
                "handoff_start_new_resident_workflow",
            ),
            (
                "Hand Over Keys & Welcome Tenant \u{2013} Leasing Agent",
                "handoff_start_new_resident_workflow",
            ),
            (
                "Hand Over Keys and Welcome Tenant - Leasing Agent",
                "handoff_start_new_resident_workflow",
            ),
            (
                "Update the unit's status in AppFolio from \"Vacant\" to \"Occupied.\"",
                "handoff_start_new_resident_workflow",
            ),
        ];

        let mut map = HashMap::with_capacity(NAME_TO_TASK.len());
        for (name, task_key) in NAME_TO_TASK {
            map.insert(normalize_name(name), *task_key);
        }
        map
    })
}

#[cfg(test)]
pub(crate) fn lookup_for_tests(name: &str) -> Option<&'static str> {
    let normalized = normalize_name(name);
    task_key_for_normalized(&normalized)
}
