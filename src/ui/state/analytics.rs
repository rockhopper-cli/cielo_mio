// src/app/state/analytics.rs

use chrono::NaiveDate;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EsgPillar {
    Environmental,
    Social,
    Governance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentalMetric {
    Scope1Emissions,
    Scope2Emissions,
    Scope3Emissions,
    RenewableEnergy,
    WaterWithdrawal,
    EnergyIntensity,
    WasteIntensity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocialMetric {
    FemaleBoardDirectors,
    GrossWagesToFemale,
    EmployeeTurnoverRate,
    WorkRelatedInjuries,
    Fatalities,
    FemaleWorkforce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceMetric {
    PoshComplaintsUpheld,
    TotalPoshComplaints,
    ComplaintsFiled,
    PendingComplaints,
    DisciplinaryActions,
    CostOnWellBeing,
}

#[derive(Debug, Clone, Default)]
pub struct CompanyEsgMetrics {
    pub entity_id: Uuid,
    pub company_name: String,
    pub corporate_id_number: String,
    pub industry_sector: String,
    pub industry_division: String,
    pub fy_start: Option<NaiveDate>,
    pub fy_end: Option<NaiveDate>,
    pub fy_label: String,

    pub scope_1_tco2e: Option<f64>,
    pub scope_2_tco2e: Option<f64>,
    pub scope_3_tco2e: Option<f64>,
    pub water_withdrawal_kl: Option<f64>,
    pub energy_intensity: Option<f64>,
    pub waste_intensity: Option<f64>,

    pub female_board_directors_pct: Option<f64>,
    pub gross_wages_to_female_pct: Option<f64>,
    pub employee_turnover_pct: Option<f64>,
    pub work_related_injuries: Option<f64>,
    pub fatalities: Option<f64>,
    pub female_workforce_pct: Option<f64>,

    pub posh_complaints_upheld: Option<f64>,
    pub total_posh_complaints: Option<f64>,
    pub complaints_filed: Option<f64>,
    pub pending_complaints: Option<f64>,
    pub disciplinary_actions: Option<f64>,
    pub cost_on_wellbeing_pct: Option<f64>,

    pub paid_up_capital_cr: Option<f64>,
    pub renewable_energy_pct: Option<f64>,
}

pub struct AnalyticsState {
    pub metrics: Vec<CompanyEsgMetrics>,
    pub unique_companies: Vec<(String, String)>,
    pub is_loading: bool,
    pub data_loaded: bool,
    pub selected_pillar: EsgPillar,
    pub selected_env_metric: EnvironmentalMetric,
    pub selected_soc_metric: SocialMetric,
    pub selected_gov_metric: GovernanceMetric,
    pub selected_cin: String,
    pub selected_sector: String,
    pub search_query: String,
    pub company_search: String,
}

impl Default for AnalyticsState {
    fn default() -> Self {
        Self {
            metrics: Vec::new(),
            unique_companies: Vec::new(),
            is_loading: false,
            data_loaded: false,
            selected_pillar: EsgPillar::Environmental,
            selected_env_metric: EnvironmentalMetric::Scope1Emissions,
            selected_soc_metric: SocialMetric::FemaleWorkforce,
            selected_gov_metric: GovernanceMetric::TotalPoshComplaints,
            selected_cin: String::new(),
            selected_sector: "All Sectors".to_string(),
            search_query: String::new(),
            company_search: String::new(),
        }
    }
}
