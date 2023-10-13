use turbo_tasks::Vc;

use super::available_chunk_items::{AvailableChunkItemInfoMap, AvailableChunkItems};

#[turbo_tasks::value(serialization = "auto_for_input")]
#[derive(PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub enum AvailabilityInfo {
    /// Availability of modules is not tracked
    Untracked,
    /// Availablility of modules is tracked, but no modules are available
    Root,
    /// There are modules already available.
    Complete {
        available_chunk_items: Vc<AvailableChunkItems>,
    },
}

impl AvailabilityInfo {
    pub fn available_chunk_items(&self) -> Option<Vc<AvailableChunkItems>> {
        match self {
            Self::Untracked => None,
            Self::Root => None,
            Self::Complete {
                available_chunk_items,
                ..
            } => Some(*available_chunk_items),
        }
    }

    pub fn with_chunk_items(self, chunk_items: Vc<AvailableChunkItemInfoMap>) -> Self {
        match self {
            AvailabilityInfo::Untracked => AvailabilityInfo::Untracked,
            AvailabilityInfo::Root => AvailabilityInfo::Complete {
                available_chunk_items: AvailableChunkItems::new(chunk_items),
            },
            AvailabilityInfo::Complete {
                available_chunk_items,
            } => AvailabilityInfo::Complete {
                available_chunk_items: available_chunk_items.with_chunk_items(chunk_items),
            },
        }
    }
}
