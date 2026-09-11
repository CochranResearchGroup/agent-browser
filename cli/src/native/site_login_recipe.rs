//! Closed, repository-owned site-login recipes and page classification.
//!
//! Tenant identity is supplied separately as an opaque run binding. A recipe
//! never contains tenant data, credentials, challenge material, or arbitrary
//! caller-provided selectors.

use super::authentication_run::{PasswordPersistencePolicy, SiteLoginState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

const BILL_LOGIN_RECIPE_JSON: &str =
    include_str!("../../../config/site-login-recipes/bill-login-v1.json");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SiteLoginRecipe {
    pub(crate) schema_version: String,
    pub(crate) recipe_id: String,
    pub(crate) target_service_id: String,
    pub(crate) allowed_origins: Vec<String>,
    pub(crate) authenticated_company_path_prefix: String,
    pub(crate) identifier: SiteFormRecipe,
    pub(crate) password: SiteFormRecipe,
    pub(crate) password_value_source: PasswordValueSource,
    pub(crate) sms_otp: SiteFormRecipe,
    pub(crate) password_persistence: PasswordPersistenceRecipe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PasswordValueSource {
    VaultOrBrowserAutofill,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SiteFormRecipe {
    pub(crate) field_selectors: Vec<String>,
    pub(crate) submit_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PasswordPersistenceRecipe {
    pub(crate) policy: PasswordPersistencePolicy,
    pub(crate) save_labels: Vec<String>,
    pub(crate) update_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SitePageEvidence {
    pub(crate) url: String,
    pub(crate) title: String,
    pub(crate) identifier_selector: Option<String>,
    pub(crate) password_selector: Option<String>,
    pub(crate) sms_otp_selector: Option<String>,
    pub(crate) visible_button_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClassifiedSitePage {
    pub(crate) state: SiteLoginState,
    pub(crate) origin_verified: bool,
    pub(crate) exact_organization_authenticated: bool,
    pub(crate) state_instance_id: String,
}

pub(crate) fn load_site_login_recipe(recipe_id: &str) -> Result<SiteLoginRecipe, String> {
    if recipe_id != "bill-login-v1" {
        return Err(format!("site_login_recipe_unsupported:{recipe_id}"));
    }
    let recipe: SiteLoginRecipe = serde_json::from_str(BILL_LOGIN_RECIPE_JSON)
        .map_err(|error| format!("site_login_recipe_invalid:{error}"))?;
    validate_recipe(&recipe)?;
    Ok(recipe)
}

pub(crate) fn site_login_recipe_digest(recipe_id: &str) -> Result<String, String> {
    let recipe = load_site_login_recipe(recipe_id)?;
    let canonical = serde_json::to_vec(&recipe)
        .map_err(|error| format!("site_login_recipe_digest_failed:{error}"))?;
    Ok(format!("{:x}", Sha256::digest(canonical)))
}

fn validate_recipe(recipe: &SiteLoginRecipe) -> Result<(), String> {
    if recipe.schema_version != "agent-browser.site-login-recipe.v1"
        || recipe.recipe_id != "bill-login-v1"
        || recipe.target_service_id != "bill"
        || recipe.allowed_origins != ["https://app.bill.com", "https://login.us.bill.com"]
        || recipe.authenticated_company_path_prefix != "/companies/"
        || recipe.identifier.field_selectors.is_empty()
        || recipe.password.field_selectors.is_empty()
        || recipe.sms_otp.field_selectors.is_empty()
        || recipe.identifier.submit_labels != ["Save", "Continue"]
        || recipe.password.submit_labels != ["Sign in"]
        || recipe.password_value_source != PasswordValueSource::VaultOrBrowserAutofill
        || recipe.sms_otp.submit_labels != ["Continue"]
        || recipe.password_persistence.policy != PasswordPersistencePolicy::Save
    {
        return Err("site_login_recipe_contract_mismatch".to_string());
    }
    Ok(())
}

pub(crate) fn classify_site_page(
    recipe: &SiteLoginRecipe,
    organization_ref: &str,
    evidence: &SitePageEvidence,
) -> Result<ClassifiedSitePage, String> {
    let parsed =
        Url::parse(&evidence.url).map_err(|_| "site_login_page_url_invalid".to_string())?;
    let origin = parsed.origin().ascii_serialization();
    let origin_verified = recipe.allowed_origins.iter().any(|item| item == &origin);
    if !origin_verified {
        return Err("site_login_origin_not_allowed".to_string());
    }
    let company_prefix = format!(
        "{}{}/",
        recipe.authenticated_company_path_prefix,
        organization_ref.trim_matches('/')
    );
    let exact_organization_authenticated = origin == "https://app.bill.com"
        && (parsed.path() == company_prefix.trim_end_matches('/')
            || parsed.path().starts_with(&company_prefix));

    let selector_allowed = |selected: &Option<String>, allowed: &[String]| {
        selected
            .as_ref()
            .is_some_and(|selector| allowed.contains(selector))
    };
    let has_button = |allowed: &[String]| {
        evidence
            .visible_button_labels
            .iter()
            .any(|label| allowed.contains(label))
    };
    let state = if exact_organization_authenticated {
        SiteLoginState::Authenticated
    } else if selector_allowed(&evidence.sms_otp_selector, &recipe.sms_otp.field_selectors)
        && has_button(&recipe.sms_otp.submit_labels)
    {
        SiteLoginState::SmsOtpForm
    } else if selector_allowed(
        &evidence.password_selector,
        &recipe.password.field_selectors,
    ) && has_button(&recipe.password.submit_labels)
    {
        SiteLoginState::PasswordForm
    } else if selector_allowed(
        &evidence.identifier_selector,
        &recipe.identifier.field_selectors,
    ) && has_button(&recipe.identifier.submit_labels)
    {
        SiteLoginState::IdentifierForm
    } else {
        SiteLoginState::UnsupportedChallenge
    };
    let canonical =
        serde_json::to_vec(&(recipe.recipe_id.as_str(), organization_ref, evidence, state))
            .map_err(|error| format!("site_login_state_digest_failed:{error}"))?;
    Ok(ClassifiedSitePage {
        state,
        origin_verified,
        exact_organization_authenticated,
        state_instance_id: format!("site-state-{:x}", Sha256::digest(canonical)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(url: &str) -> SitePageEvidence {
        SitePageEvidence {
            url: url.to_string(),
            title: "BILL".to_string(),
            identifier_selector: None,
            password_selector: None,
            sms_otp_selector: None,
            visible_button_labels: Vec::new(),
        }
    }

    #[test]
    fn bill_recipe_is_closed_and_has_stable_digest() {
        let recipe = load_site_login_recipe("bill-login-v1").unwrap();
        assert_eq!(recipe.target_service_id, "bill");
        let digest = site_login_recipe_digest("bill-login-v1").unwrap();
        assert_eq!(digest.len(), 64);
        assert_eq!(digest, site_login_recipe_digest("bill-login-v1").unwrap());
        assert!(load_site_login_recipe("caller-supplied-recipe").is_err());
    }

    #[test]
    fn authenticated_requires_exact_bill_company_path() {
        let recipe = load_site_login_recipe("bill-login-v1").unwrap();
        let exact = classify_site_page(
            &recipe,
            "company-123",
            &evidence("https://app.bill.com/companies/company-123/spend/transactions"),
        )
        .unwrap();
        assert_eq!(exact.state, SiteLoginState::Authenticated);
        assert!(exact.exact_organization_authenticated);

        let wrong = classify_site_page(
            &recipe,
            "company-123",
            &evidence("https://app.bill.com/companies/company-999/spend/transactions"),
        )
        .unwrap();
        assert_eq!(wrong.state, SiteLoginState::UnsupportedChallenge);
        assert!(!wrong.exact_organization_authenticated);
    }

    #[test]
    fn forms_are_classified_only_from_closed_selectors_and_labels() {
        let recipe = load_site_login_recipe("bill-login-v1").unwrap();
        let mut current_identifier = evidence("https://login.us.bill.com/neo/login");
        current_identifier.identifier_selector =
            Some("input#login-email-input[name='loginEmail']".to_string());
        current_identifier
            .visible_button_labels
            .push("Continue".to_string());
        assert_eq!(
            classify_site_page(&recipe, "company-123", &current_identifier)
                .unwrap()
                .state,
            SiteLoginState::IdentifierForm
        );

        let mut legacy_identifier = evidence("https://login.us.bill.com/neo/login");
        legacy_identifier.identifier_selector = Some("input[type='email']".to_string());
        legacy_identifier
            .visible_button_labels
            .push("Save".to_string());
        assert_eq!(
            classify_site_page(&recipe, "company-123", &legacy_identifier)
                .unwrap()
                .state,
            SiteLoginState::IdentifierForm
        );

        let mut otp = evidence("https://login.us.bill.com/auth");
        otp.sms_otp_selector = Some("input[autocomplete='one-time-code']".to_string());
        otp.visible_button_labels.push("Continue".to_string());
        assert_eq!(
            classify_site_page(&recipe, "company-123", &otp)
                .unwrap()
                .state,
            SiteLoginState::SmsOtpForm
        );

        otp.sms_otp_selector = Some("input[data-caller-selector]".to_string());
        assert_eq!(
            classify_site_page(&recipe, "company-123", &otp)
                .unwrap()
                .state,
            SiteLoginState::UnsupportedChallenge
        );
    }

    #[test]
    fn non_bill_origin_fails_closed() {
        let recipe = load_site_login_recipe("bill-login-v1").unwrap();
        assert_eq!(
            classify_site_page(
                &recipe,
                "company-123",
                &evidence("https://example.invalid/companies/company-123")
            )
            .unwrap_err(),
            "site_login_origin_not_allowed"
        );
    }
}
