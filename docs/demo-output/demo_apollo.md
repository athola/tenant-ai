Agentic workflow demo
Vacancy workflow demo
Vacancy window: 2025-09-28 -> 2025-10-12 (evaluated 2025-09-28)
Data source: Apollo CSV import

Stage progress
- Marketing & Advertising: 0/2 tasks completed
- Screening & Application: 0/3 tasks completed
- Lease Signing & Move-In: 0/4 tasks completed
- Handoff: 0/1 tasks completed

Role workload
- Leasing Agent: 6 open, 0 overdue
- Compliance Coordinator: 1 open, 0 overdue
- Property Manager: 2 open, 0 overdue
- Property Manager (Accounting): 1 open, 0 overdue

Overdue tasks: none

Compliance alerts
- [Warning] Iowa Code § 562A.29 reasonable re-rental efforts: Document every marketing channel touch to evidence reasonable efforts to re-rent (Iowa Code § 562A.29).
- [Warning] System of record accuracy: Accurate AppFolio statuses keep vacancy analytics, owner reporting, and marketing automation in sync.
- [Warning] Fair Housing and Iowa Civil Rights Act parity: Consistent response cadences prevent disparate treatment across protected classes and leave an audit trail.
- [Warning] Documented screening criteria: Apply published screening criteria uniformly and retain documentation for adverse action defense.
- [Warning] LIHTC source-of-income verification: Secure third-party income documentation to support Tenant Income Certification (TIC) files.
- [Warning] Adverse action documentation: Retain copies of denial notices and credit disclosures to satisfy Fair Credit Reporting Act obligations.
- [Warning] Lease artifact completeness: Incomplete lease packets jeopardize move-in readiness and downstream LIHTC audits.
- [Warning] Security deposit limits: Deposits exceeding state limits expose the portfolio to statutory penalties.
- [Warning] Move-in condition documentation: Thorough inspections limit security deposit disputes and support future turn charges.
- [Warning] LIHTC eligibility lock-in: Certification must be finalized at least three days before move-in to maintain LIHTC compliance.
- [Warning] Operational handoff completeness: Transitioning to onboarding ensures services, compliance tracking, and resident engagement continue seamlessly.

Readiness score: 0% (Monitor)
Expected pace 0% | Days since vacancy 0 | Days until move-in 14
Focus stage: Lease Signing & Move-In (0% complete)

AI observations
- 0 of 10 tasks complete (0% readiness)

Recommended actions
- Concentrate automation on Lease Signing & Move-In (4 open items)
- Bundle lease packet tasks and push DocuSign reminders automatically
- Escalate compliance checklist to coordinator with documented follow-up

Automation triggers
- Auto-remind Marketing & Advertising owners of 2 remaining tasks
- Auto-remind Screening & Application owners of 3 remaining tasks
- Auto-remind Lease Signing & Move-In owners of 4 remaining tasks
- Auto-remind Handoff owners of 1 remaining task

Communication automation snapshot (last 7 days)
- 42 inbound leads | 83% automated first touch coverage
- SLA target 5 min | actual 2.8 min avg response | 93% SLA adherence
- 7 conversations escalated to humans after automation
Channel mix:
  - SMS: 19 leads | 89% automated | 1.4 min avg | 97% SLA | 2 live assist escalations
  - Email: 15 leads | 80% automated | 4.6 min avg | 88% SLA | 3 live assist escalations
  - Voice: 8 leads | 75% automated | 2.9 min avg | 91% SLA | 2 live assist escalations

Application intake demo (sensitive fields redacted)
- Received application app-000001 -> status submitted
  Decision rationale: pending evaluation
  Evaluation decision: denied for insufficient income (required 0.28, actual 0.28) (score 0)
  Household summary: 2 adults / 1 children (2 bedrooms)
  Score components (lawful factors only):
    - RentToIncome: -40 (ratio 0.28 exceeds required 0.28)
    - CreditScore: 20 (credit score 705 meets minimum 650)
    - RentalHistory: 10 (no prior evictions)
    - VoucherCoverage: 5 (voucher covers 38% of rent)
    - IowaSecurityDepositCompliance: 5 (security deposit within Iowa cap)
  Rent-to-income ratio (inputs redacted): 0.28
  Public status payload:
{
  "application_id": "app-000001",
  "status": "denied",
  "decision_rationale": "denied for insufficient income (required 0.28, actual 0.28)",
  "total_score": 0
}
  External alerts: none dispatched
