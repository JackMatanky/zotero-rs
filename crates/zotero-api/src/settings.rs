//! Settings management API wrapper.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    client::{ZoteroClient, ZoteroResponse},
    errors::ZoteroApiError,
};

/// Key-value setting payload returned by the Zotero settings API.
///
/// Zotero setting keys identify individual preferences or extension settings.
/// The `value` field is JSON-typed because settings can be strings, booleans,
/// numbers, arrays, or objects depending on the key.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SettingEntry {
    /// Setting key name.
    pub key: String,
    /// Setting value as raw JSON.
    pub value: serde_json::Value,
}

impl ZoteroClient {
    /// Fetches all settings for the target library.
    ///
    /// Returns a [`HashMap`] keyed by setting name. The Local API may return
    /// settings either as an object keyed by setting name or as a list of
    /// [`SettingEntry`] values. This method accepts both shapes and normalizes
    /// them into one map.
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

    /// Fetches one setting by key.
    ///
    /// Requests `/settings/{key}` and returns a [`SettingEntry`]. If Zotero
    /// returns only the setting value instead of a full setting object, the
    /// requested key is paired with that JSON value.
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

    /// Updates one setting value by key.
    ///
    /// Sends the JSON `value` to `/settings/{key}`. Zotero decides whether the
    /// key can be created or updated. If the key does not exist or cannot be
    /// written, the server response is returned as
    /// [`ZoteroApiError::LocalApi`].
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
