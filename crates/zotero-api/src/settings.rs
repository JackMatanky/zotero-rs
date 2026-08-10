//! Settings management API wrapper.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    client::{ZoteroClient, ZoteroResponse},
    errors::ZoteroApiError,
};

/// Setting entry payload for client configuration settings.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SettingEntry {
    /// Setting key name.
    pub key: String,
    /// Setting value payload.
    pub value: serde_json::Value,
}

impl ZoteroClient {
    /// Fetches all configuration settings for the target library.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero returns a non-2xx status
    /// - [`Network`] on connection failure
    /// - [`Json`] if the response cannot be deserialized
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    /// [`Json`]: ZoteroApiError::Json
    #[inline]
    #[expect(clippy::else_if_without_else, reason = "fallback list handling")]
    pub async fn get_settings(
        &self,
    ) -> Result<HashMap<String, SettingEntry>, ZoteroApiError> {
        let res: ZoteroResponse<serde_json::Value> =
            self.get("/settings").send().await?;
        let raw = res.data;
        let mut result = HashMap::new();
        if let Some(obj) = raw.as_object() {
            for (k, v) in obj {
                result.insert(k.clone(), SettingEntry {
                    key: k.clone(),
                    value: v.clone(),
                });
            }
        } else if let Ok(list) =
            serde_json::from_value::<Vec<SettingEntry>>(raw)
        {
            for item in list {
                result.insert(item.key.clone(), item);
            }
        }
        Ok(result)
    }

    /// Fetches a single setting entry by setting key name.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero returns a non-2xx status
    /// - [`Network`] on connection failure
    /// - [`Json`] if the response cannot be deserialized
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    /// [`Json`]: ZoteroApiError::Json
    #[inline]
    pub async fn get_setting<K: AsRef<str>>(
        &self,
        key: K,
    ) -> Result<SettingEntry, ZoteroApiError> {
        let k = key.as_ref();
        let res: ZoteroResponse<serde_json::Value> =
            self.get(format!("/settings/{k}")).send().await?;
        let val = res.data;
        if let Ok(entry) = serde_json::from_value::<SettingEntry>(val.clone()) {
            Ok(entry)
        } else {
            Ok(SettingEntry {
                key: k.to_owned(),
                value: val,
            })
        }
    }

    /// Updates a setting value by key name.
    ///
    /// # Errors
    ///
    /// - [`LocalApi`] if Zotero rejects the setting update
    /// - [`Network`] on connection failure
    ///
    /// [`LocalApi`]: ZoteroApiError::LocalApi
    /// [`Network`]: ZoteroApiError::Network
    #[inline]
    pub async fn update_setting<K: AsRef<str>>(
        &self,
        key: K,
        value: serde_json::Value,
    ) -> Result<(), ZoteroApiError> {
        let k = key.as_ref();
        self.put(format!("/settings/{k}")).json(value).send_unit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::client::test_http::{MockServer, http_response};

    #[tokio::test]
    async fn parses_get_settings_map_response() {
        let json_resp = serde_json::json!({
            "export.quickCopy.setting": "as-bibtex",
            "sync.auto": true
        })
        .to_string();

        let server = MockServer::new(vec![http_response("200 OK", &json_resp)]);
        let client = ZoteroClient::new(server.url());

        let settings = client.get_settings().await.unwrap();
        assert_eq!(settings.len(), 2);
    }
}
