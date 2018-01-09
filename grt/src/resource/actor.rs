use std::io::{Error, ErrorKind};
use std::rc::Rc;

use resource::{Item, ResourceBuilder, ResourceSet, EntitySize, Sprite};

use serde_json;
use serde_yaml;

pub struct Actor {
    pub id: String,
    pub player: bool,
    pub text_display: char,
    pub image_display: Rc<Sprite>,
    pub default_size: Rc<EntitySize>,
    pub items: Vec<Rc<Item>>,
}

impl PartialEq for Actor {
    fn eq(&self, other: &Actor) -> bool {
        self.id == other.id
    }
}

impl Actor {
    pub fn new(builder: ActorBuilder, resources: &ResourceSet) -> Result<Actor, Error> {

        let default_size = match resources.sizes.get(&builder.default_size) {
            None => {
                warn!("No match found for size '{}'", builder.default_size);
                return Err(Error::new(ErrorKind::InvalidData,
                    format!("Unable to create actor '{}'", builder.id)));
            },
            Some(size) => Rc::clone(size)
        };


        let mut items: Vec<Rc<Item>> = Vec::new();
        if let Some(builder_items) = builder.items {
            for item_id in builder_items {
                let item = match resources.items.get(&item_id) {
                    None => {
                        warn!("No match found for item ID '{}'", item_id);
                        return Err(Error::new(ErrorKind::InvalidData,
                             format!("Unable to create actor '{}'", builder.id)));
                    },
                    Some(item) => Rc::clone(item)
                };
                items.push(item);
            }
        }

        let sprite = resources.get_sprite(&builder.image_display)?;

        Ok(Actor {
            id: builder.id,
            player: builder.player.unwrap_or(false),
            text_display: builder.text_display,
            image_display: sprite,
            default_size: default_size,
            items,
        })
    }
}

#[derive(Deserialize, Debug)]
pub struct ActorBuilder {
    pub id: String,
    pub player: Option<bool>,
    pub text_display: char,
    pub image_display: String,
    pub default_size: usize,
    pub items: Option<Vec<String>>,
}

impl ResourceBuilder for ActorBuilder {
    fn owned_id(&self) -> String {
        self.id.to_owned()
    }

    fn from_json(data: &str) -> Result<ActorBuilder, Error> {
        let resource: ActorBuilder = serde_json::from_str(data)?;

        Ok(resource)
    }

    fn from_yaml(data: &str) -> Result<ActorBuilder, Error> {
        let resource: Result<ActorBuilder, serde_yaml::Error> = serde_yaml::from_str(data);

        match resource {
            Ok(resource) => Ok(resource),
            Err(error) => Err(Error::new(ErrorKind::InvalidData, format!("{}", error)))
        }
    }
}