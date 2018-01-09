mod path_finder_grid;
use self::path_finder_grid::PathFinderGrid;

use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::rc::Rc;

use resource::{Terrain, ResourceBuilder, ResourceSet, Sprite};
use util::{Point, Size};

use serde_json;
use serde_yaml;

#[derive(Debug)]
pub struct Transition {
    pub from: Point,
    pub size: Size,
    pub to: Point,
    pub to_area: Option<String>,
    pub image_display: Rc<Sprite>,
    pub text_display: char,
}

#[derive(Deserialize, Debug)]
pub struct ActorData {
    pub id: String,
    pub location: Point,
}

pub struct Area {
    pub id: String,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub terrain: Terrain,
    path_grids: HashMap<i32, PathFinderGrid>,
    pub actors: Vec<ActorData>,
    pub transitions: Vec<Transition>,
}

impl PartialEq for Area {
    fn eq(&self, other: &Area) -> bool {
        self.id == other.id
    }
}

impl Area {
    pub fn new(builder: AreaBuilder, resources: &ResourceSet) -> Result<Area, Error> {
        debug!("Creating area {}", builder.id);
        let terrain = Terrain::new(&builder, &resources.tiles);
        let terrain = match terrain {
            Ok(l) => l,
            Err(e) => {
                warn!("Unable to generate terrain for area '{}'", builder.id);
                return Err(e);
            }
        };

        let mut path_grids: HashMap<i32, PathFinderGrid> = HashMap::new();
        for size in resources.sizes.values() {
            let path_grid = PathFinderGrid::new(Rc::clone(size), &terrain);
            trace!("Generated path grid of size {}", size.size);
            path_grids.insert(size.size, path_grid);
        }

        // TODO validate position of each actor

        let mut transitions: Vec<Transition> = Vec::new();
        for (index, t_builder) in builder.transitions.into_iter().enumerate() {
            let sprite = resources.get_sprite(&t_builder.image_display)?;

            let mut p = t_builder.from;
            if !p.in_bounds(builder.width as i32, builder.height as i32) {
                warn!("Transition {} falls outside area bounds", index);
                continue;
            }
            p.add(t_builder.size.width, t_builder.size.height);
            if !p.in_bounds(builder.width as i32, builder.height as i32) {
                warn!("Transition {} falls outside area bounds", index);
                continue;
            }

            let transition = Transition {
                from: t_builder.from,
                to: t_builder.to,
                size: t_builder.size,
                to_area: t_builder.to_area,
                text_display: t_builder.text_display,
                image_display: sprite,
            };
            transitions.push(transition);
        }

        Ok(Area {
            id: builder.id,
            name: builder.name,
            width: builder.width as i32,
            height: builder.height as i32,
            terrain: terrain,
            path_grids: path_grids,
            actors: builder.actors,
            transitions,
        })
    }

    pub fn coords_valid(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 { return false; }
        if x >= self.width || y >= self.height { return false; }

        true
    }

    pub fn get_path_grid(&self, size: i32) -> &PathFinderGrid {
        self.path_grids.get(&size).unwrap()
    }
}

#[derive(Deserialize, Debug)]
pub struct AreaBuilder {
    pub id: String,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub terrain: HashMap<String, Vec<Vec<usize>>>,
    pub generate: bool,
    actors: Vec<ActorData>,
    transitions: Vec<TransitionBuilder>,
}

impl ResourceBuilder for AreaBuilder {
    fn owned_id(&self) -> String {
        self.id.to_owned()
    }

    fn from_json(data: &str) -> Result<AreaBuilder, Error> {
        let resource: AreaBuilder = serde_json::from_str(data)?;

        Ok(resource)
    }

    fn from_yaml(data: &str) -> Result<AreaBuilder, Error> {
        let resource: Result<AreaBuilder, serde_yaml::Error> = serde_yaml::from_str(data);

        match resource {
            Ok(resource) => Ok(resource),
            Err(error) => Err(Error::new(ErrorKind::InvalidData, format!("{}", error)))
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct TransitionBuilder {
    pub from: Point,
    pub size: Size,
    pub to: Point,
    pub to_area: Option<String>,
    pub image_display: String,
    pub text_display: char,
}