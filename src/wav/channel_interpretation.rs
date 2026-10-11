//! User-selected interpretation of WAV channel indices, independent of sample loading.
//!
//! Raw header metadata remains the caller's property. Resolving a preset only describes
//! roles and expected padding; it never inspects, reorders, or changes audio.

use crate::audio::{Id, Layout};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelOrderPreset {
    #[default]
    Automatic,
    AuroIdOrder,
    Unassigned,
}

/// Retain this recipe across loads, rather than retaining a previously resolved mapping.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelInterpretation {
    pub preset: ChannelOrderPreset,
    /// Optional user-selected active layout for Auro ID order.
    #[serde(default, with = "layout_bits")]
    pub active_layout: Option<Layout>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpretationSource {
    Wavex,
    AuroIdOrder { assumed: bool },
    Unassigned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelRole {
    /// Potential speaker at this index, including indices marked as padding.
    pub id: Option<Id>,
    /// Expected padding according to a selected layout, not a measurement of silence.
    pub padding: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedChannelInterpretation {
    pub source: InterpretationSource,
    /// Indexed by the original WAV index, including channels not selected for loading.
    pub channels: Vec<ChannelRole>,
    /// Header-derived for WAVEX; user-selected for Auro ID order; otherwise unknown.
    pub active_layout: Option<Layout>,
    /// Distinct compatible catalog masks, with names/aliases available through `Layout::names`.
    pub candidate_layouts: Vec<Layout>,
}

impl ChannelInterpretation {
    /// Clear a selected layout that is no longer compatible with the header or preset.
    pub fn resolve_and_validate(
        &mut self,
        channel_mask: Option<u32>,
        total_nr_channels: usize,
    ) -> ResolvedChannelInterpretation {
        let resolved = self.resolve(channel_mask, total_nr_channels);
        self.active_layout = if matches!(resolved.source, InterpretationSource::AuroIdOrder { .. })
        {
            resolved.active_layout.clone()
        } else {
            None
        };
        resolved
    }

    /// Resolve against the total header channel count, not the number of loaded buffers.
    /// Incompatible explicit layouts are ignored; callers can clear them using this result.
    pub fn resolve(
        &self,
        channel_mask: Option<u32>,
        total_nr_channels: usize,
    ) -> ResolvedChannelInterpretation {
        let wavex_layout = super::channel_mask::layout(channel_mask, total_nr_channels);
        let source = match self.preset {
            ChannelOrderPreset::Automatic if wavex_layout.is_some() => InterpretationSource::Wavex,
            ChannelOrderPreset::Automatic if channel_mask.is_none_or(|mask| mask == 0) => {
                InterpretationSource::AuroIdOrder { assumed: true }
            }
            ChannelOrderPreset::AuroIdOrder => InterpretationSource::AuroIdOrder { assumed: false },
            _ => InterpretationSource::Unassigned,
        };
        let candidate_layouts = if matches!(source, InterpretationSource::AuroIdOrder { .. }) {
            Layout::catalog()
                .iter()
                .filter(|layout| {
                    !layout.is_empty()
                        && (u64::BITS - layout.bits().leading_zeros()) as usize == total_nr_channels
                })
                .cloned()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let active_layout = match source {
            InterpretationSource::Wavex => wavex_layout,
            InterpretationSource::AuroIdOrder { .. } => self
                .active_layout
                .as_ref()
                .filter(|layout| candidate_layouts.contains(layout))
                .cloned(),
            InterpretationSource::Unassigned => None,
        };
        let mut channels = vec![
            ChannelRole {
                id: None,
                padding: false,
            };
            total_nr_channels
        ];
        match source {
            InterpretationSource::Wavex => {
                for (index, channel) in channels.iter_mut().enumerate() {
                    channel.id =
                        super::channel_mask::channel_id(channel_mask, total_nr_channels, index);
                }
            }
            InterpretationSource::AuroIdOrder { .. } => {
                // Match explicit values rather than relying on enum or JSON iteration order.
                for id in Id::iter() {
                    if let Some(channel) = channels.get_mut(id as usize) {
                        channel.id = Some(id);
                    }
                }
                if let Some(layout) = &active_layout {
                    for (index, channel) in channels.iter_mut().enumerate() {
                        channel.padding = layout.bits() & (1u64 << index) == 0;
                    }
                }
            }
            InterpretationSource::Unassigned => {}
        }
        ResolvedChannelInterpretation {
            source,
            channels,
            active_layout,
            candidate_layouts,
        }
    }
}

// Serialize the selection as an internal layout mask, independently of bitflags formatting.
mod layout_bits {
    use super::*;

    pub fn serialize<S: serde::Serializer>(
        layout: &Option<Layout>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        layout.as_ref().map(Layout::bits).serialize(serializer)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Layout>, D::Error> {
        Ok(Option::<u64>::deserialize(deserializer)?.map(Layout::from_bits_retain))
    }
}

#[cfg(test)]
#[path = "channel_interpretation_tests.rs"]
mod tests;
