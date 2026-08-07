//! Vector representation, dot-product scoring, BLOB encoding, and local ONNX
//! embedding.

use std::{path::Path, sync::Mutex};

use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use zotero_api::ZoteroApiError;

use crate::EmbeddingProvider;

const MODEL: EmbeddingModel = EmbeddingModel::BGESmallENV15;
const EMBED_BATCH_SIZE: usize = 32;

/// Production [`EmbeddingProvider`] backed by a local ONNX model via
/// `fastembed`.
pub struct FastEmbedProvider {
    model: Mutex<TextEmbedding>,
}

impl FastEmbedProvider {
    /// Loads the fixed embedding model, downloading it to `cache_dir` if
    /// needed.
    ///
    /// # Errors
    ///
    /// Returns [`ZoteroApiError::Embedding`] if model loading or downloading
    /// fails.
    #[inline]
    pub fn load(cache_dir: &Path) -> Result<Self, ZoteroApiError> {
        let options = TextInitOptions::new(MODEL)
            .with_cache_dir(cache_dir.to_path_buf())
            .with_show_download_progress(false);
        let model = TextEmbedding::try_new(options)
            .map_err(|e| ZoteroApiError::Embedding(e.to_string()))?;
        Ok(Self {
            model: Mutex::new(model),
        })
    }
}

impl std::fmt::Debug for FastEmbedProvider {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FastEmbedProvider").finish_non_exhaustive()
    }
}

impl EmbeddingProvider for FastEmbedProvider {
    #[inline]
    fn embed(
        &self,
        texts: &[String],
    ) -> Result<Vec<Embedding>, ZoteroApiError> {
        let mut model = self.model.lock().map_err(|_| {
            ZoteroApiError::Embedding(
                "embedding model mutex poisoned".to_owned(),
            )
        })?;
        let vectors: Vec<Vec<f32>> = model
            .embed(texts, Some(EMBED_BATCH_SIZE))
            .map_err(|e| ZoteroApiError::Embedding(e.to_string()))?;
        Ok(vectors.into_iter().map(Embedding::from).collect())
    }
}

/// A dense embedding vector produced by the model and stored in the index.
#[derive(Clone, Debug, PartialEq)]
pub struct Embedding(Vec<f32>);

impl Embedding {
    /// L2-normalizes the vector in place.
    #[inline]
    pub fn normalize(&mut self) {
        let norm_sq: f32 = self.0.iter().map(|x| x * x).sum();
        if norm_sq <= 0.0 {
            return;
        }
        let norm = norm_sq.sqrt();
        for x in &mut self.0 {
            *x /= norm;
        }
    }

    /// Calculates the dot product of two equal-length prenormalized vectors.
    #[must_use]
    #[inline]
    pub fn dot(&self, other: &Embedding) -> f32 {
        if self.0.len() != other.0.len() {
            return 0.0;
        }
        self.0.iter().zip(&other.0).map(|(x, y)| x * y).sum()
    }

    /// Encodes the vector as little-endian `f32` bytes for `BLOB` storage.
    #[must_use]
    #[inline]
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.0.len().saturating_mul(4));
        for value in &self.0 {
            buf.extend_from_slice(&value.to_le_bytes());
        }
        buf
    }
}

impl From<Vec<f32>> for Embedding {
    #[inline]
    fn from(values: Vec<f32>) -> Self {
        Self(values)
    }
}

impl TryFrom<&[u8]> for Embedding {
    type Error = ZoteroApiError;

    #[inline]
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (chunks, remainder) = bytes.as_chunks::<4>();
        if !remainder.is_empty() {
            return Err(ZoteroApiError::Embedding(
                "corrupt embedding blob: length is not a multiple of 4"
                    .to_owned(),
            ));
        }
        let values = chunks.iter().map(|c| f32::from_le_bytes(*c)).collect();
        Ok(Self(values))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn encode_decode_round_trips_including_negative_and_zero() {
        let original = Embedding::from(vec![0.0, -1.5, 3.25, -0.000_1, 42.0]);
        let encoded = original.encode();
        let decoded = Embedding::try_from(encoded.as_slice()).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_rejects_non_multiple_of_four_length() {
        let bytes = vec![0_u8, 1, 2];
        assert!(Embedding::try_from(bytes.as_slice()).is_err());
    }

    #[test]
    fn normalize_leaves_zero_vector_unchanged() {
        let mut vector = Embedding::from(vec![0.0_f32, 0.0, 0.0]);
        vector.normalize();
        assert_eq!(vector, Embedding::from(vec![0.0, 0.0, 0.0]));
    }

    #[test]
    fn normalized_self_similarity_is_approximately_one() {
        let mut vector = Embedding::from(vec![1.0_f32, 2.0, 3.0, -4.0]);
        vector.normalize();
        let similarity = vector.dot(&vector);
        assert!(
            (similarity - 1.0).abs() < 1e-6,
            "expected ~1.0, got {similarity}"
        );
    }

    #[test]
    fn dot_mismatched_lengths_returns_zero() {
        let a = Embedding::from(vec![1.0_f32, 2.0]);
        let b = Embedding::from(vec![1.0_f32, 2.0, 3.0]);
        assert!(a.dot(&b).abs() < f32::EPSILON);
    }
}
