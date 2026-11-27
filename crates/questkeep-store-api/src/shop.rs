//! The "Buy from" and "Activate on" entries of a store's "Add game" menu, with the store option
//! that chooses between its launcher and the browser.

use crate::model::AddGameOption;
use crate::settings::StoreOption;

/// The store option (a checkbox, unticked by default) that opens the shop and key redemption in
/// the browser instead of the launcher.
pub const OPEN_IN_BROWSER: &str = "openStorePagesInBrowser";

/// Id of the "Buy from" entry.
pub const BUY_ID: &str = "shop";
/// Id of the "Activate on" entry.
pub const ACTIVATE_ID: &str = "redeem";

/// Where a store sells games and redeems product keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShopPages {
    /// The shop's website (https).
    pub shop_url: &'static str,
    /// A store URI that opens the shop in the launcher.
    pub shop_uri: Option<&'static str>,
    /// The web page where product keys are redeemed (https).
    pub redeem_url: Option<&'static str>,
    /// A store URI that opens the launcher's key redemption.
    pub redeem_uri: Option<&'static str>,
    /// Keys are redeemed in the launcher only, from its own menu: "Activate on" just opens it.
    pub redeem_in_launcher: bool,
}

impl ShopPages {
    /// A store whose shop is a website and that has no key redemption.
    pub const fn web(shop_url: &'static str) -> Self {
        Self {
            shop_url,
            shop_uri: None,
            redeem_url: None,
            redeem_uri: None,
            redeem_in_launcher: false,
        }
    }

    /// The launcher-or-browser option, for stores whose launcher can open one of the pages.
    pub fn option(&self) -> Option<StoreOption> {
        let (label, entries) = match (self.shop_uri, self.redeem_uri) {
            (Some(_), Some(_)) => (
                "Open the store and key activation in the browser",
                "\"Buy from\" and \"Activate on\"",
            ),
            (Some(_), None) => ("Open the store in the browser", "\"Buy from\""),
            (None, Some(_)) => ("Open key activation in the browser", "\"Activate on\""),
            (None, None) => return None,
        };
        Some(StoreOption {
            key: OPEN_IN_BROWSER.into(),
            label: label.into(),
            hint: format!(
                "Otherwise {entries} in the Add game menu open in the launcher while it is installed."
            ),
        })
    }

    /// The "Buy from" and "Activate on" entries for the store `name`. They use the launcher while
    /// it is `installed` and [`OPEN_IN_BROWSER`] isn't ticked in `config`, the browser otherwise.
    pub fn entries(
        &self,
        name: &str,
        installed: bool,
        config: &serde_json::Value,
    ) -> Vec<AddGameOption> {
        let launcher =
            installed && config.get(OPEN_IN_BROWSER) != Some(&serde_json::Value::Bool(true));
        let buy = AddGameOption::buy(BUY_ID, name);
        let buy = match self.shop_uri {
            Some(uri) if launcher => buy.opening_uri(uri),
            _ => buy.opening_browser(self.shop_url),
        };
        let activate = AddGameOption::activate(ACTIVATE_ID, name);
        let activate = match (self.redeem_uri, self.redeem_url) {
            (Some(uri), _) if launcher => Some(activate.opening_uri(uri)),
            (_, Some(url)) => Some(activate.opening_browser(url)),
            _ if self.redeem_in_launcher => Some(activate.opening_launcher("")),
            _ => None,
        };
        std::iter::once(buy).chain(activate).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AddGameSection, AddGameTarget};
    use serde_json::{Value, json};

    const PAGES: ShopPages = ShopPages {
        shop_url: "https://shop.example/",
        shop_uri: Some("store://shop"),
        redeem_url: Some("https://shop.example/redeem"),
        redeem_uri: Some("store://redeem"),
        redeem_in_launcher: false,
    };

    fn targets(options: &[AddGameOption]) -> Vec<(AddGameSection, AddGameTarget)> {
        options
            .iter()
            .map(|o| (o.section, o.opens.clone()))
            .collect()
    }

    fn uri(uri: &str) -> AddGameTarget {
        AddGameTarget::Uri { uri: uri.into() }
    }

    fn web(url: &str) -> AddGameTarget {
        AddGameTarget::Browser { url: url.into() }
    }

    #[test]
    fn the_launcher_is_used_unless_the_browser_is_chosen_or_it_is_missing() {
        let launcher = PAGES.entries("Example", true, &Value::Null);
        assert_eq!(
            targets(&launcher),
            [
                (AddGameSection::Buy, uri("store://shop")),
                (AddGameSection::Activate, uri("store://redeem"))
            ]
        );
        assert!(launcher.iter().all(|o| o.label == "Example"));
        let browser = [
            (AddGameSection::Buy, web("https://shop.example/")),
            (AddGameSection::Activate, web("https://shop.example/redeem")),
        ];
        let chosen = PAGES.entries("Example", true, &json!({ "openStorePagesInBrowser": true }));
        assert_eq!(targets(&chosen), browser);
        assert_eq!(
            targets(&PAGES.entries("Example", false, &Value::Null)),
            browser
        );
        assert!(PAGES.option().is_some());
    }

    #[test]
    fn web_only_stores_have_no_option_and_maybe_no_redemption() {
        let pages = ShopPages::web("https://shop.example/");
        assert!(pages.option().is_none());
        assert_eq!(
            targets(&pages.entries("Example", true, &Value::Null)),
            [(AddGameSection::Buy, web("https://shop.example/"))]
        );
        let in_launcher = ShopPages {
            redeem_in_launcher: true,
            ..pages
        };
        assert_eq!(
            targets(&in_launcher.entries("Example", true, &Value::Null))[1],
            (
                AddGameSection::Activate,
                AddGameTarget::Launcher {
                    args: String::new(),
                    keys: None
                }
            )
        );
    }
}
