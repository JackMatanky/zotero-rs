//! Security profiles, path allowlists, and input size constraints.

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

use zotero_api::ZoteroApiError;

const DEFAULT_MAX_PDF_BYTES: u64 = 50 * 1024 * 1024;
const DEFAULT_MAX_HTTP_BODY_BYTES: usize = 10 * 1024 * 1024;
const DEFAULT_MAX_MARKDOWN_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_MAX_HTML_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_MAX_TEMPLATE_NAME_BYTES: usize = 128;
const HARDENED_MAX_PDF_BYTES: u64 = 25 * 1024 * 1024;
const HARDENED_MAX_HTTP_BODY_BYTES: usize = 2 * 1024 * 1024;
const HARDENED_MAX_MARKDOWN_BYTES: usize = 512 * 1024;
const HARDENED_MAX_HTML_BYTES: usize = 512 * 1024;

/// Security profiles supported by the Zotero MCP server.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecurityProfile {
    /// Default profile: conservative read-only access.
    Default,
    /// Workspace profile: allows reading and exports relative to CWD.
    Workspace,
    /// Trusted local profile: allows reading from standard user paths.
    TrustedLocal,
    /// Hardened profile: restricts maximum request/response sizes.
    Hardened,
}

/// Security configuration parameters controlling path access and size limits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityConfig {
    profile: SecurityProfile,
    direct_file_paths: bool,
    file_paths_enabled: bool,
    allowed_read_dirs: Vec<PathBuf>,
    allowed_aux_dirs: Vec<PathBuf>,
    allowed_export_dirs: Vec<PathBuf>,
    max_pdf_bytes: u64,
    max_http_body_bytes: usize,
    max_markdown_bytes: usize,
    max_html_bytes: usize,
    max_template_name_bytes: usize,
}

impl Default for SecurityConfig {
    #[inline]
    fn default() -> Self {
        Self {
            profile: SecurityProfile::Default,
            direct_file_paths: false,
            file_paths_enabled: false,
            allowed_read_dirs: Vec::new(),
            allowed_aux_dirs: Vec::new(),
            allowed_export_dirs: Vec::new(),
            max_pdf_bytes: DEFAULT_MAX_PDF_BYTES,
            max_http_body_bytes: DEFAULT_MAX_HTTP_BODY_BYTES,
            max_markdown_bytes: DEFAULT_MAX_MARKDOWN_BYTES,
            max_html_bytes: DEFAULT_MAX_HTML_BYTES,
            max_template_name_bytes: DEFAULT_MAX_TEMPLATE_NAME_BYTES,
        }
    }
}

impl SecurityConfig {
    /// Reads security configuration from environment variables.
    #[expect(
        clippy::disallowed_methods,
        reason = "profile defaults intentionally use the process working \
                  directory"
    )]
    #[inline]
    pub fn from_env() -> Self {
        let current_dir =
            env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let home_dir = env::var_os("HOME").map(PathBuf::from);
        Self::from_sources(
            |name| env::var_os(name),
            &current_dir,
            home_dir.as_deref(),
        )
    }

    fn from_sources<F>(
        mut get_var: F,
        current_dir: &Path,
        home_dir: Option<&Path>,
    ) -> Self
    where
        F: FnMut(&str) -> Option<OsString>,
    {
        let profile = get_var("ZOTERO_MCP_PROFILE")
            .and_then(|v| v.into_string().ok())
            .and_then(|v| match v.as_str() {
                "workspace" => Some(SecurityProfile::Workspace),
                "trusted-local" => Some(SecurityProfile::TrustedLocal),
                "hardened" => Some(SecurityProfile::Hardened),
                "default" => Some(SecurityProfile::Default),
                _ => None,
            })
            .unwrap_or(SecurityProfile::Default);

        let mut config = match profile {
            SecurityProfile::Default => Self::default(),
            SecurityProfile::Workspace => Self {
                profile,
                direct_file_paths: true,
                file_paths_enabled: true,
                allowed_read_dirs: vec![current_dir.to_path_buf()],
                allowed_aux_dirs: vec![current_dir.to_path_buf()],
                allowed_export_dirs: vec![current_dir.join("exports")],
                ..Self::default()
            },
            SecurityProfile::TrustedLocal => {
                let mut config = Self {
                    profile,
                    direct_file_paths: true,
                    file_paths_enabled: true,
                    ..Self::default()
                };
                if let Some(home) = home_dir {
                    config.allowed_read_dirs = vec![
                        home.join("Documents"),
                        home.join("Downloads"),
                        home.join("Zotero/storage"),
                    ];
                    config.allowed_aux_dirs =
                        vec![home.join("Documents"), home.join("Downloads")];
                    config.allowed_export_dirs =
                        vec![home.join("Documents/Zotero Exports")];
                }
                config
            }
            SecurityProfile::Hardened => Self {
                profile,
                max_pdf_bytes: HARDENED_MAX_PDF_BYTES,
                max_http_body_bytes: HARDENED_MAX_HTTP_BODY_BYTES,
                max_markdown_bytes: HARDENED_MAX_MARKDOWN_BYTES,
                max_html_bytes: HARDENED_MAX_HTML_BYTES,
                ..Self::default()
            },
        };

        if let Some(value) =
            get_var("ZOTERO_DIRECT_FILE_PATHS").and_then(parse_bool)
        {
            config.direct_file_paths = value;
        }
        if let Some(value) =
            get_var("ZOTERO_FILE_PATHS_ENABLED").and_then(parse_bool)
        {
            config.file_paths_enabled = value;
        }
        if let Some(value) = get_var("ZOTERO_ALLOWED_READ_DIRS") {
            config.allowed_read_dirs = env::split_paths(&value).collect();
        }
        if let Some(value) = get_var("ZOTERO_ALLOWED_AUX_DIRS") {
            config.allowed_aux_dirs = env::split_paths(&value).collect();
        }
        if let Some(value) = get_var("ZOTERO_ALLOWED_EXPORT_DIRS") {
            config.allowed_export_dirs = env::split_paths(&value).collect();
        }
        if let Some(value) = get_var("ZOTERO_MAX_PDF_BYTES").and_then(parse_u64)
        {
            config.max_pdf_bytes = value;
        }
        if let Some(value) =
            get_var("ZOTERO_MAX_HTTP_BODY_BYTES").and_then(parse_usize)
        {
            config.max_http_body_bytes = value;
        }
        if let Some(value) =
            get_var("ZOTERO_MAX_MARKDOWN_BYTES").and_then(parse_usize)
        {
            config.max_markdown_bytes = value;
        }
        if let Some(value) =
            get_var("ZOTERO_MAX_HTML_BYTES").and_then(parse_usize)
        {
            config.max_html_bytes = value;
        }
        if let Some(value) =
            get_var("ZOTERO_MAX_TEMPLATE_NAME_BYTES").and_then(parse_usize)
        {
            config.max_template_name_bytes = value;
        }
        config
    }

    /// Checks if direct filepath access is enabled by policy.
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::InputRejected`] if direct filepath access is
    /// disabled by policy.
    #[inline]
    pub fn check_direct_file_paths_enabled(
        &self,
    ) -> Result<(), ZoteroApiError> {
        if self.is_direct_file_paths_enabled() {
            Ok(())
        } else {
            Err(ZoteroApiError::InputRejected(
                "Direct file paths are disabled; set \
                 ZOTERO_MCP_PROFILE=workspace or \
                 ZOTERO_DIRECT_FILE_PATHS=true with ZOTERO_ALLOWED_READ_DIRS"
                    .to_owned(),
            ))
        }
    }

    /// Validates that a path exists and falls under one of the allowed `roots`.
    #[expect(
        clippy::disallowed_methods,
        reason = "canonicalization is the security boundary for symlink-safe \
                  reads"
    )]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::InputRejected`] if `path` is not inside an
    /// allowed root directory.
    #[inline]
    pub fn check_existing_read_path<'a, I>(
        &self,
        path: &Path,
        roots: I,
        purpose: &str,
    ) -> Result<PathBuf, ZoteroApiError>
    where
        I: IntoIterator<Item = &'a PathBuf>,
    {
        let checked = std::fs::canonicalize(path)?;
        if path_is_allowed(&checked, roots) {
            Ok(checked)
        } else {
            Err(ZoteroApiError::InputRejected(format!(
                "{purpose} path {} is outside allowed directories",
                checked.display()
            )))
        }
    }

    /// Validates that an output `path` target directory is allowed for writes.
    #[expect(
        clippy::disallowed_methods,
        reason = "canonicalization is the security boundary for symlink-safe \
                  outputs"
    )]
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::InputRejected`] if output parent directory is
    /// missing or not inside allowed `roots`.
    #[inline]
    pub fn check_output_path(
        &self,
        path: &Path,
        roots: &[PathBuf],
        purpose: &str,
    ) -> Result<PathBuf, ZoteroApiError> {
        let Some(parent) = path.parent() else {
            return Err(ZoteroApiError::InputRejected(format!(
                "{purpose} parent directory is missing"
            )));
        };
        let parent = std::fs::canonicalize(parent).map_err(|_| {
            ZoteroApiError::InputRejected(format!(
                "{purpose} parent directory is missing"
            ))
        })?;
        if !path_is_allowed(&parent, roots) {
            return Err(ZoteroApiError::InputRejected(format!(
                "{purpose} path {} is outside allowed directories",
                path.display()
            )));
        }
        let file_name = path.file_name().ok_or_else(|| {
            ZoteroApiError::InputRejected(format!(
                "{purpose} output file name is missing"
            ))
        })?;
        Ok(parent.join(file_name))
    }

    /// Checks that `path` points to a `.pdf` file within maximum allowed byte
    /// limits.
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::InputRejected`] if `path` lacks a `.pdf`
    /// extension or exceeds `max_pdf_bytes`.
    #[inline]
    pub fn check_pdf_file(&self, path: &Path) -> Result<(), ZoteroApiError> {
        let is_pdf = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));
        if !is_pdf {
            return Err(ZoteroApiError::InputRejected(format!(
                "PDF read path must have a .pdf extension: {}",
                path.display()
            )));
        }
        let len = std::fs::metadata(path)?.len();
        if len > self.max_pdf_bytes {
            return Err(ZoteroApiError::InputRejected(format!(
                "PDF file {} exceeds {} bytes",
                path.display(),
                self.max_pdf_bytes
            )));
        }
        Ok(())
    }

    /// Validates that `markdown` content does not exceed `max_markdown_bytes`.
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::InputRejected`] if size exceeds the configured
    /// maximum limit.
    #[inline]
    pub fn check_markdown_size(
        &self,
        markdown: &str,
    ) -> Result<(), ZoteroApiError> {
        check_text_size(markdown, self.max_markdown_bytes, "markdown")
    }

    /// Validates that `html` content does not exceed `max_html_bytes`.
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::InputRejected`] if size exceeds the configured
    /// maximum limit.
    #[inline]
    pub fn check_html_size(&self, html: &str) -> Result<(), ZoteroApiError> {
        check_text_size(html, self.max_html_bytes, "HTML")
    }

    /// Validates that template `name` does not exceed
    /// `max_template_name_bytes`.
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::InputRejected`] if size exceeds the configured
    /// maximum limit.
    #[inline]
    pub fn check_template_name_size(
        &self,
        name: &str,
    ) -> Result<(), ZoteroApiError> {
        check_text_size(name, self.max_template_name_bytes, "template name")
    }

    #[must_use]
    #[inline]
    pub fn profile(&self) -> SecurityProfile {
        self.profile
    }

    #[must_use]
    #[inline]
    pub fn is_direct_file_paths_enabled(&self) -> bool {
        self.direct_file_paths
    }

    #[must_use]
    #[inline]
    pub fn is_file_paths_enabled(&self) -> bool {
        self.file_paths_enabled
    }

    #[must_use]
    #[inline]
    pub fn allowed_read_dirs(&self) -> &[PathBuf] {
        &self.allowed_read_dirs
    }

    #[must_use]
    #[inline]
    pub fn allowed_aux_dirs(&self) -> &[PathBuf] {
        &self.allowed_aux_dirs
    }

    #[must_use]
    #[inline]
    pub fn allowed_export_dirs(&self) -> &[PathBuf] {
        &self.allowed_export_dirs
    }

    #[must_use]
    #[inline]
    pub fn max_pdf_bytes(&self) -> u64 {
        self.max_pdf_bytes
    }

    #[must_use]
    #[inline]
    pub fn max_http_body_bytes(&self) -> usize {
        self.max_http_body_bytes
    }

    #[must_use]
    #[inline]
    pub fn max_markdown_bytes(&self) -> usize {
        self.max_markdown_bytes
    }

    #[must_use]
    #[inline]
    pub fn max_html_bytes(&self) -> usize {
        self.max_html_bytes
    }

    #[must_use]
    #[inline]
    pub fn max_template_name_bytes(&self) -> usize {
        self.max_template_name_bytes
    }

    /// Validates an AUX path, ensuring file path features are enabled.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::InputRejected`] if file path features are
    /// disabled or `path` is outside allowed directories.
    #[inline]
    pub fn check_aux_path(
        &self,
        path: &Path,
    ) -> Result<PathBuf, ZoteroApiError> {
        if !self.is_file_paths_enabled() {
            return Err(ZoteroApiError::InputRejected(
                "File path features are disabled; set \
                 ZOTERO_MCP_PROFILE=workspace or \
                 ZOTERO_FILE_PATHS_ENABLED=true"
                    .to_owned(),
            ));
        }
        self.check_existing_read_path(path, &self.allowed_aux_dirs, "AUX scan")
    }
}

impl SecurityConfig {
    #[inline]
    pub fn set_direct_file_paths_enabled(&mut self, enabled: bool) {
        self.direct_file_paths = enabled;
    }

    #[inline]
    pub fn set_file_paths_enabled(&mut self, enabled: bool) {
        self.file_paths_enabled = enabled;
    }

    #[inline]
    pub fn set_allowed_read_dirs(&mut self, dirs: Vec<PathBuf>) {
        self.allowed_read_dirs = dirs;
    }

    #[inline]
    pub fn set_allowed_export_dirs(&mut self, dirs: Vec<PathBuf>) {
        self.allowed_export_dirs = dirs;
    }

    #[inline]
    pub fn set_allowed_aux_dirs(&mut self, dirs: Vec<PathBuf>) {
        self.allowed_aux_dirs = dirs;
    }

    #[inline]
    pub fn set_max_pdf_bytes(&mut self, max: u64) {
        self.max_pdf_bytes = max;
    }

    #[inline]
    pub fn set_max_http_body_bytes(&mut self, max: usize) {
        self.max_http_body_bytes = max;
    }

    #[inline]
    pub fn set_max_markdown_bytes(&mut self, max: usize) {
        self.max_markdown_bytes = max;
    }

    #[inline]
    pub fn set_max_html_bytes(&mut self, max: usize) {
        self.max_html_bytes = max;
    }

    #[inline]
    pub fn set_max_template_name_bytes(&mut self, max: usize) {
        self.max_template_name_bytes = max;
    }
}

fn parse_bool(value: OsString) -> Option<bool> {
    let value = value.into_string().ok()?;
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" => Some(true),
        "0" | "false" | "no" | "n" => Some(false),
        _ => None,
    }
}

fn parse_u64(value: OsString) -> Option<u64> {
    value.into_string().ok()?.parse().ok()
}

fn parse_usize(value: OsString) -> Option<usize> {
    value.into_string().ok()?.parse().ok()
}

#[expect(
    clippy::disallowed_methods,
    reason = "allowed-root comparisons must use canonical paths"
)]
fn path_is_allowed<'a, I>(path: &Path, roots: I) -> bool
where
    I: IntoIterator<Item = &'a PathBuf>,
{
    roots
        .into_iter()
        .filter_map(|root| std::fs::canonicalize(root).ok())
        .any(|root| path.starts_with(root))
}

fn check_text_size(
    value: &str,
    max_bytes: usize,
    field: &str,
) -> Result<(), ZoteroApiError> {
    if value.len() > max_bytes {
        Err(ZoteroApiError::InputRejected(format!(
            "{field} exceeds {max_bytes} bytes"
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::Path};

    use pretty_assertions::assert_eq;

    use super::*;

    fn config_from<'a>(
        vars: &'a [(&'a str, &'a str)],
        current_dir: &Path,
        home_dir: Option<&Path>,
    ) -> SecurityConfig {
        SecurityConfig::from_sources(
            |name| {
                vars.iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| OsString::from(value))
            },
            current_dir,
            home_dir,
        )
    }

    #[test]
    fn verifies_security_config_getters_and_setters() {
        let mut config = SecurityConfig::default();
        assert_eq!(config.profile(), SecurityProfile::Default);
        assert_eq!(config.is_direct_file_paths_enabled(), false);
        assert_eq!(config.is_file_paths_enabled(), false);
        assert!(config.allowed_read_dirs().is_empty());
        assert!(config.allowed_aux_dirs().is_empty());
        assert!(config.allowed_export_dirs().is_empty());
        assert_eq!(config.max_pdf_bytes(), 50 * 1024 * 1024);
        assert_eq!(config.max_http_body_bytes(), 10 * 1024 * 1024);
        assert_eq!(config.max_markdown_bytes(), 2 * 1024 * 1024);
        assert_eq!(config.max_html_bytes(), 2 * 1024 * 1024);
        assert_eq!(config.max_template_name_bytes(), 128);

        config.set_file_paths_enabled(true);
        config.set_allowed_read_dirs(vec![PathBuf::from("/read")]);
        config.set_allowed_aux_dirs(vec![PathBuf::from("/aux")]);
        config.set_allowed_export_dirs(vec![PathBuf::from("/export")]);
        config.set_max_pdf_bytes(100);
        config.set_max_http_body_bytes(200);
        config.set_max_markdown_bytes(300);
        config.set_max_html_bytes(400);
        config.set_max_template_name_bytes(50);

        assert_eq!(config.is_file_paths_enabled(), true);
        assert_eq!(config.allowed_read_dirs(), &[PathBuf::from("/read")]);
        assert_eq!(config.allowed_aux_dirs(), &[PathBuf::from("/aux")]);
        assert_eq!(config.allowed_export_dirs(), &[PathBuf::from("/export")]);
        assert_eq!(config.max_pdf_bytes(), 100);
        assert_eq!(config.max_http_body_bytes(), 200);
        assert_eq!(config.max_markdown_bytes(), 300);
        assert_eq!(config.max_html_bytes(), 400);
        assert_eq!(config.max_template_name_bytes(), 50);
    }

    #[test]
    fn default_profile_disables_direct_and_file_paths() {
        let current_dir = Path::new("/work/project");

        let config = config_from(&[], current_dir, None);

        assert_eq!(config.profile, SecurityProfile::Default);
        assert!(!config.direct_file_paths);
        assert!(!config.file_paths_enabled);
        assert!(config.allowed_read_dirs.is_empty());
        assert!(config.allowed_aux_dirs.is_empty());
        assert!(config.allowed_export_dirs.is_empty());
        assert_eq!(config.max_pdf_bytes, 50 * 1024 * 1024);
        assert_eq!(config.max_http_body_bytes, 10 * 1024 * 1024);
        assert_eq!(config.max_markdown_bytes, 2 * 1024 * 1024);
        assert_eq!(config.max_html_bytes, 2 * 1024 * 1024);
        assert_eq!(config.max_template_name_bytes, 128);
    }
}
