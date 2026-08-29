use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::fixture::color::ColorType;
use crate::fixture::{ChannelError, FixtureError, FIXTURE_TYPE_LIST};
use crate::fixture::channel::{ChannelParameter, PropertyType, SimplePropertyType};

/// A template defining the DMX-Channel layout for a type of lighting fixture.
///
/// Fixture types are registered globally and used to create [`crate::fixture::Fixture`] instances.
/// See [`FixtureType::new`] for how properties are parsed and validated.
#[derive(Debug,Serialize,Deserialize)]
pub struct FixtureType {
    pub(super) color: Option<ColorType>,
    pub(super) properties: HashMap<SimplePropertyType, ChannelParameter>,
    pub(super) name: String,
}

impl FixtureType {
    /// Creates a new fixture type and registers it globally.
    ///
    /// Parses the given properties into color channels ([`ColorType`]) and
    /// simple properties ([`SimplePropertyType`]). Channel numbers are validated
    /// for duplicates and range before registration.
    ///
    /// # Usage
    ///
    /// Register a fixture type first with [`FixtureType::new`], then create
    /// instances of it with [`crate::fixture::Fixture::new`].
    ///
    /// # Arguments
    ///
    /// * `name`        - Unique name for this fixture type.
    /// * `properties` - Map of property types to their corresponding [`ChannelParameter`] layout.
    ///
    /// # Errors
    ///
    /// * [`InvalidPropertyType`](FixtureError::InvalidPropertyType) – if a property name is not recognized.
    /// * [`FixtureTypeNameAlreadyInUse`](FixtureError::FixtureTypeNameAlreadyInUse) – if the name is already 
    /// registered.
    /// * [`ChannelAlreadyInUse`](ChannelError::ChannelAlreadyInUse) – if two properties share a channel.
    /// * [`ChannelOutOfRange`](ChannelError::ChannelOutOfRange) – if a channel exceeds [`MAX_CHANNEL`].

    pub fn new(
        name: String,
        properties: HashMap<PropertyType, ChannelParameter>,
    ) -> Result<(), FixtureError> {
        let mut color = ColorType::new();
        let mut new_properties = HashMap::new();
        let mut seen_channels = HashSet::new();

        for (key, channels) in &properties {
            for channel in channels.get_channel_indices() {
                if !seen_channels.insert(channel) {
                    return Err(FixtureError::ChannelError(
                        ChannelError::ChannelAlreadyInUse(key.to_string()),
                    ));
                }
            }

            match key {
                PropertyType::Color(color_type) => {
                    color.checked_add_channel(color_type.clone(), channels.clone())?;
                }
                PropertyType::Simple(simple_property) => {
                    new_properties.insert(simple_property.clone(), channels.clone());
                }
            }
        }

        let color = if color.exists() { Some(color) } else { None };

        let output = Self {
            color,
            properties: new_properties,
            name: name.clone(),
        };

        let mut list = FIXTURE_TYPE_LIST.write().unwrap();
        match list.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(_) => {
                Err(FixtureError::FixtureTypeNameAlreadyInUse(name))
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(output);
                Ok(())
            }
        }
    }
}
