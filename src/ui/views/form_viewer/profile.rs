// src/ui/views/form_viewer/profile.rs

use super::formatter::format_decimal_with_commas;
use crate::ui::state::{CompanyFilingSummary, FormMasterItem};
use std::borrow::Cow;

pub fn resolve_profile_value<'a>(
    item: &FormMasterItem,
    profile: Option<&'a CompanyFilingSummary>,
) -> Option<Cow<'a, str>> {
    let prof = profile?;
    let text = item.question_text.as_str();
    let key = item.question_key.as_str();
    let tag = item.standard_indicator_capmkt.as_str();

    if tag == "CorporateIdentityNumber"
        || key.contains("q01_")
        || text.contains("Corporate Identity Number")
    {
        return Some(Cow::Borrowed(prof.corporate_id_number.as_str()));
    }
    if tag == "NameOfTheCompany"
        || text.contains("Name of the Entity")
        || text.contains("Name of the Company")
    {
        return Some(Cow::Borrowed(prof.company_name.as_str()));
    }
    if (tag == "AddressOfRegisteredOfficeOfCompany" || text.contains("Registered office address"))
        && let Some(ref addr) = prof.address_registered_office
    {
        return Some(Cow::Borrowed(addr.as_str()));
    }
    if (tag == "AddressOfCorporateOfficeOfCompany"
        || text.contains("Corporate address")
        || text.contains("Corporate office address"))
        && let Some(ref addr) = prof.address_corporate_office
    {
        return Some(Cow::Borrowed(addr.as_str()));
    }
    if (tag == "EMailOfTheCompany" || text.contains("E-mail") || text == "Email")
        && let Some(ref em) = prof.company_email
    {
        return Some(Cow::Borrowed(em.as_str()));
    }
    if (tag == "TelephoneOfCompany" || text == "Telephone")
        && let Some(ref ph) = prof.company_telephone
    {
        return Some(Cow::Borrowed(ph.as_str()));
    }
    if (tag == "WebsiteOfCompany" || text.contains("Website"))
        && let Some(ref web) = prof.website
    {
        return Some(Cow::Borrowed(web.as_str()));
    }
    if (tag == "ValueOfSharesPaidUp"
        || text.contains("Paid-up Capital")
        || text.contains("Paid up Capital"))
        && let Some(val) = prof.paid_up_shares_value
    {
        return Some(Cow::Owned(format!("₹ {}", format_decimal_with_commas(val))));
    }
    if text.contains("Financial year for which reporting")
        && let (Some(s), Some(e)) = (prof.fy_start, prof.fy_end)
    {
        return Some(Cow::Owned(format!("{} to {}", s, e)));
    }

    None
}
