// src/app/tab_form_viewer/profile.rs

use crate::app::state::{CompanyFilingSummary, FormMasterItem};
use std::borrow::Cow;

pub fn resolve_profile_value<'a>(
    item: &FormMasterItem,
    profile: Option<&'a CompanyFilingSummary>,
) -> Option<Cow<'a, str>> {
    let prof = profile?;
    let text = &item.question_text;
    let key = &item.question_key;

    if key.contains("cin") || text.contains("Corporate Identity Number") {
        return Some(Cow::Borrowed(prof.corporate_id_number.as_str()));
    }
    if text.contains("Name of the Company") {
        return Some(Cow::Borrowed(prof.company_name.as_str()));
    }
    if text.contains("Registered office address") {
        if let Some(ref addr) = prof.address_registered_office {
            return Some(Cow::Borrowed(addr.as_str()));
        }
    }
    if text.contains("Corporate office address") {
        if let Some(ref addr) = prof.address_corporate_office {
            return Some(Cow::Borrowed(addr.as_str()));
        }
    }
    if text.contains("Email") || text.contains("E-mail") {
        if let Some(ref em) = prof.company_email {
            return Some(Cow::Borrowed(em.as_str()));
        }
    }
    if text.contains("Telephone") {
        if let Some(ref ph) = prof.company_telephone {
            return Some(Cow::Borrowed(ph.as_str()));
        }
    }
    if text.contains("Website") {
        if let Some(ref web) = prof.website {
            return Some(Cow::Borrowed(web.as_str()));
        }
    }
    if text.contains("Paid up Capital") || text.contains("Paid-up") {
        if let Some(val) = prof.paid_up_shares_value {
            return Some(Cow::Owned(format!("₹ {}", val)));
        }
    }
    if text.contains("Financial year") {
        if let (Some(s), Some(e)) = (prof.fy_start, prof.fy_end) {
            return Some(Cow::Owned(format!("{} to {}", s, e)));
        }
    }

    None
}
