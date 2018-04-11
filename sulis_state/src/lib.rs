//  This file is part of Sulis, a turn based RPG written in Rust.
//  Copyright 2018 Jared Stephen
//
//  Sulis is free software: you can redistribute it and/or modify
//  it under the terms of the GNU General Public License as published by
//  the Free Software Foundation, either version 3 of the License, or
//  (at your option) any later version.
//
//  Sulis is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU General Public License for more details.
//
//  You should have received a copy of the GNU General Public License
//  along with Sulis.  If not, see <http://www.gnu.org/licenses/>

extern crate rlua;
extern crate rand;

extern crate sulis_core;
extern crate sulis_module;
extern crate sulis_rules;
#[macro_use] extern crate log;

mod ai;
pub use self::ai::AI;

mod ability_state;
pub use self::ability_state::AbilityState;

pub mod animation;
use self::animation::{Animation, MoveAnimation};

mod area_feedback_text;
use self::area_feedback_text::AreaFeedbackText;

mod area_state;
pub use self::area_state::AreaState;

mod change_listener;
pub use self::change_listener::ChangeListener;
pub use self::change_listener::ChangeListenerList;

mod effect;
pub use self::effect::Effect;

mod entity_state;
pub use self::entity_state::EntityState;
pub use self::entity_state::AreaDrawable;

mod entity_texture_cache;
pub use self::entity_texture_cache::EntityTextureCache;
pub use self::entity_texture_cache::EntityTextureSlot;

mod actor_state;
pub use self::actor_state::ActorState;

mod item_state;
pub use self::item_state::ItemState;

pub mod inventory;
pub use self::inventory::Inventory;

pub mod item_list;
pub use self::item_list::ItemList;

mod location;
pub use self::location::Location;

mod los_calculator;
pub use self::los_calculator::calculate_los;
pub use self::los_calculator::has_visibility;

mod merchant;
pub use self::merchant::Merchant;

mod path_finder;
use self::path_finder::PathFinder;

mod prop_state;
pub use self::prop_state::PropState;

mod script;
pub use self::script::ScriptState;
pub use self::script::targeter::Targeter;
pub use self::script::ScriptCallback;

mod turn_timer;
pub use self::turn_timer::TurnTimer;
pub use self::turn_timer::ROUND_TIME_MILLIS;

use std::time;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::rc::Rc;
use std::cell::RefCell;

use sulis_rules::HitKind;
use sulis_core::config::CONFIG;
use sulis_core::util::{self, Point};
use sulis_core::io::{GraphicsRenderer, MainLoopUpdater};
use sulis_core::ui::Widget;
use sulis_module::{Ability, Actor, Module};

use script::ScriptEntitySet;
use script::script_callback::ScriptHitKind;

thread_local! {
    static STATE: RefCell<Option<GameState>> = RefCell::new(None);
    static AI: RefCell<AI> = RefCell::new(AI::new());
    static ENTERING_COMBAT: RefCell<bool> = RefCell::new(false);
    static SCRIPT: ScriptState = ScriptState::new();
    static ANIMATIONS: RefCell<Vec<Box<Animation>>> = RefCell::new(Vec::new());
    static ANIMS_TO_ADD: RefCell<Vec<Box<Animation>>> = RefCell::new(Vec::new());
}

pub struct GameStateMainLoopUpdater { }

impl MainLoopUpdater for GameStateMainLoopUpdater {
    fn update(&self, root: &Rc<RefCell<Widget>>, millis: u32) {
        GameState::update(root, millis);
    }

    fn is_exit(&self) -> bool {
        GameState::is_exit()
    }
}

pub struct GameState {
    areas: HashMap<String, Rc<RefCell<AreaState>>>,
    area_state: Rc<RefCell<AreaState>>,
    pc: Rc<RefCell<EntityState>>,
    should_exit: bool,
    path_finder: PathFinder,
}

macro_rules! exec_script {
    ($func:ident: $($x:ident),*) => {
        let start_time = time::Instant::now();

        let result: Result<(), rlua::Error> = SCRIPT.with(|script_state| {
            script_state.$func($($x, )*)
        });

        if let Err(e) = result {
            warn!("Error executing lua script function");
            warn!("{}", e);
        }

        info!("Script execution time: {}", util::format_elapsed_secs(start_time.elapsed()));
    }
}

impl GameState {
    pub fn execute_ability_on_activate(parent: &Rc<RefCell<EntityState>>, ability: &Rc<Ability>) {
        exec_script!(ability_on_activate: parent, ability);
    }

    pub fn execute_ability_on_target_select(parent: &Rc<RefCell<EntityState>>, ability: &Rc<Ability>,
                                            targets: Vec<Option<Rc<RefCell<EntityState>>>>,
                                            selected_point: Point) {
        exec_script!(ability_on_target_select: parent, ability, targets, selected_point);
    }

    pub fn execute_ability_after_attack(parent: &Rc<RefCell<EntityState>>, ability: &Rc<Ability>,
                                        targets: ScriptEntitySet,
                                        kind: HitKind, func: &str) {
        let hit_kind = ScriptHitKind { kind };
        let t = Some(("hit", hit_kind));
        exec_script!(ability_script: parent, ability, targets, t, func);
    }

    pub fn execute_ability_script(parent: &Rc<RefCell<EntityState>>, ability: &Rc<Ability>,
                                  targets: ScriptEntitySet, func: &str) {
        let t: Option<(&str, usize)> = None;
        exec_script!(ability_script: parent, ability, targets, t, func);
    }

    pub fn init(pc_actor: Rc<Actor>) -> Result<(), Error> {
        let game_state = GameState::new(pc_actor)?;

        STATE.with(|state| {
            *state.borrow_mut() = Some(game_state);
        });

        Ok(())
    }

    pub fn transition(area_id: &Option<String>, x: i32, y: i32) {
        let p = Point::new(x, y);
        info!("Area transition to {:?} at {},{}", area_id, x, y);

        if let &Some(ref area_id) = area_id {
            // check if area state has already been loaded
            let area_state = GameState::get_area_state(area_id);
            let area_state = match area_state {
                Some(area_state) => area_state,
                None => match GameState::setup_area_state(area_id) {
                    // area state has not already been loaded, try to load it
                    Ok(area_state) => {
                        STATE.with(|state| {
                            let mut state = state.borrow_mut();
                            let state = state.as_mut().unwrap();
                            state.areas.insert(area_id.to_string(), Rc::clone(&area_state));
                        });

                        area_state
                    }, Err(e) => {
                        error!("Unable to transition to '{}'", &area_id);
                        error!("{}", e);
                        return;
                    }
                }
            };

            if !GameState::check_location(&p, &area_state) {
                return;
            }

            STATE.with(|state| {
                let path_finder = PathFinder::new(&area_state.borrow().area);
                state.borrow_mut().as_mut().unwrap().path_finder = path_finder;
                state.borrow_mut().as_mut().unwrap().area_state = area_state;
            });
        } else {
            if !GameState::check_location(&p, &GameState::area_state()) {
                return;
            }
        }

        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let state = state.as_mut().unwrap();

            {
                let area_id = state.pc.borrow().location.area_id.to_string();
                state.areas.get(&area_id).unwrap().borrow_mut().remove_entity(&state.pc);
            }

            let location = Location::new(x, y, &state.area_state.borrow().area);
            state.area_state.borrow_mut().add_entity(Rc::clone(&state.pc), location);
            state.area_state.borrow_mut().push_scroll_to_callback(Rc::clone(&state.pc));

            let area_state = state.area_state.borrow();
            for entity in area_state.entity_iter() {
                entity.borrow_mut().clear_texture_cache();
            }
        });
    }

    fn check_location(p: &Point, area_state: &Rc<RefCell<AreaState>>) -> bool {
        let location = Location::from_point(p, &area_state.borrow().area);
        if !location.coords_valid(location.x, location.y) {
            error!("Location coordinates {},{} are not valid for area {}",
                   location.x, location.y, location.area_id);
            return false;
        }

        true
    }

    fn setup_area_state(area_id: &str) -> Result<Rc<RefCell<AreaState>>, Error> {
        debug!("Setting up area state from {}", &area_id);

        let area = Module::area(&area_id);
        let area = match area {
            Some(a) => a,
            None => {
                error!("Area '{}' not found", &area_id);
                return Err(Error::new(ErrorKind::NotFound, "Unable to create area."));
            }
        };
        let area_state = Rc::new(RefCell::new(AreaState::new(area)));
        area_state.borrow_mut().populate();

        Ok(area_state)
    }

    fn new(pc: Rc<Actor>) -> Result<GameState, Error> {
        let game = Module::game();

        let area_state = GameState::setup_area_state(&game.starting_area)?;

        debug!("Setting up PC {}, with {:?}", &pc.name, &game.starting_location);
        let location = Location::from_point(&game.starting_location, &area_state.borrow().area);

        if !location.coords_valid(location.x, location.y) {
            error!("Starting location coordinates must be valid for the starting area.");
            return Err(Error::new(ErrorKind::InvalidData,
                                  "Unable to create starting location."));
        }

        if !area_state.borrow_mut().add_actor(pc, location, true, None) {
            error!("Player character starting location must be within \
                   area bounds and passable.");
            return Err(Error::new(ErrorKind::InvalidData,
                "Unable to add player character to starting area at starting location"));
        }

        let pc_state = Rc::clone(area_state.borrow().get_last_entity().unwrap());
        pc_state.borrow_mut().actor.init_turn();

        let path_finder = PathFinder::new(&area_state.borrow().area);

        let mut areas: HashMap<String, Rc<RefCell<AreaState>>> = HashMap::new();
        areas.insert(game.starting_area.to_string(), Rc::clone(&area_state));

        Ok(GameState {
            areas,
            area_state: area_state,
            path_finder: path_finder,
            pc: pc_state,
            should_exit: false,
        })
    }

    fn check_clear_entering_combat() -> bool {
        ENTERING_COMBAT.with(|c| {
            let retval = {
                *c.borrow()
            };

            *c.borrow_mut() = false;
            retval
        })
    }

    pub fn set_entering_combat() {
        ENTERING_COMBAT.with(|c| *c.borrow_mut() = true);
    }

    fn get_area_state(id: &str) -> Option<Rc<RefCell<AreaState>>> {
        STATE.with(|s| {
            match s.borrow().as_ref().unwrap().areas.get(id) {
                None => None,
                Some(area_state) => Some(Rc::clone(&area_state)),
            }
        })
    }

    pub fn is_exit() -> bool {
        STATE.with(|s| s.borrow().as_ref().unwrap().should_exit)
    }

    pub fn set_exit() -> bool {
        trace!("Setting state exit flag.");
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().should_exit = true);
        true
    }

    pub fn area_state() -> Rc<RefCell<AreaState>> {
        STATE.with(|s| Rc::clone(&s.borrow().as_ref().unwrap().area_state))
    }

    pub fn update(root: &Rc<RefCell<Widget>>, millis: u32) {
        let mut anims_to_add: Vec<Box<Animation>> = ANIMS_TO_ADD.with(|a| {
            let mut anims = a.borrow_mut();

            let to_add = anims.drain(0..).collect();

            to_add
        });

        ANIMATIONS.with(|a| {
            let mut anims = a.borrow_mut();

            anims.append(&mut anims_to_add);

            let mut i = 0;
            while i < anims.len() {
                let retain = anims[i].update(root);

                if retain {
                    i += 1;
                } else {
                    anims.remove(i);
                }
            }
        });

        let active_entity = STATE.with(|s| {
            let mut state = s.borrow_mut();
            let state = state.as_mut().unwrap();
            let mut area_state = state.area_state.borrow_mut();

            let result = match area_state.update(millis) {
                None => Rc::clone(&state.pc),
                Some(entity) => Rc::clone(entity),
            };

            if state.pc.borrow().actor.is_dead() {
                area_state.turn_timer.set_active(false);
            }
            result
        });

        // clear animations for the active entity when entering combat
        if GameState::check_clear_entering_combat() {
            ANIMATIONS.with(|a| {
                let mut anims = a.borrow_mut();
                anims.iter_mut().for_each(|a| a.check(&active_entity));
            });
        }

        AI.with(|ai| {
            let mut ai = ai.borrow_mut();
            ai.update(active_entity);
        });
    }

    pub fn draw_graphics_mode(renderer: &mut GraphicsRenderer, offset_x: f32, offset_y: f32,
                              scale_x: f32, scale_y: f32, millis: u32) {
        ANIMATIONS.with(|a| {
            let anims = a.borrow();

            for anim in anims.iter() {
                anim.draw_graphics_mode(renderer, offset_x, offset_y, scale_x, scale_y, millis);
            }
        })
    }

    pub fn has_active_animations(entity: &Rc<RefCell<EntityState>>) -> bool {
        ANIMATIONS.with(|a| {
            let anims = a.borrow();

            for anim in anims.iter() {
                if *anim.get_owner().borrow() == *entity.borrow() {
                    return true;
                }
            }
            false
        })
    }

    pub fn add_animation(anim: Box<Animation>) {
        ANIMS_TO_ADD.with(|a| {
            let mut anims = a.borrow_mut();

            anims.push(anim);
        });
    }

    /// Returns true if the game is currently in turn mode, false otherwise
    pub fn is_in_turn_mode() -> bool {
        let area_state = GameState::area_state();
        let area_state = area_state.borrow();
        area_state.turn_timer.is_active()
    }

    /// Returns true if the PC has the current turn, false otherwise
    pub fn is_pc_current() -> bool {
        let area_state = GameState::area_state();
        let area_state = area_state.borrow();
        if let Some(entity) = area_state.turn_timer.current() {
            return entity.borrow().is_pc();
        }
        false
    }

    pub fn is_current(entity: &Rc<RefCell<EntityState>>) -> bool {
        let area_state = GameState::area_state();
        let area_state = area_state.borrow();
        if let Some(ref current) = area_state.turn_timer.current() {
            return Rc::ptr_eq(current, entity);
        }
        false
    }

    pub fn pc() -> Rc<RefCell<EntityState>> {
        STATE.with(|s| Rc::clone(&s.borrow().as_ref().unwrap().pc))
    }

    fn get_target(entity: &Rc<RefCell<EntityState>>,
                  target: &Rc<RefCell<EntityState>>) -> (f32, f32, f32) {
        let (target_x, target_y) = {
            let target = target.borrow();
            (target.location.x as f32 + target.size.width as f32 / 2.0 - 0.5,
             target.location.y as f32 + target.size.height as f32/ 2.0 - 0.5)
        };

        let sizes = (entity.borrow().size.diagonal + target.borrow().size.diagonal) / 2.0;
        let mut range = sizes + entity.borrow().actor.stats.attack_distance();

        let area = GameState::area_state();
        let vis_dist = area.borrow().area.vis_dist as f32;
        if range > vis_dist {
            range = vis_dist;
        }

        trace!("Getting move target at {}, {} within {}", target_x, target_y, range);
        (target_x, target_y, range)
    }

    pub fn can_move_towards(entity: &Rc<RefCell<EntityState>>,
                            target: &Rc<RefCell<EntityState>>) -> bool {
        let (x, y, dist) = GameState::get_target(entity, target);
        GameState::can_move_towards_point(entity, x, y, dist)
    }

    pub fn move_towards(entity: &Rc<RefCell<EntityState>>,
                        target: &Rc<RefCell<EntityState>>) -> bool {
        let (x, y, dist) = GameState::get_target(entity, target);
        GameState::move_towards_point(entity, x, y, dist, None)
    }

    pub fn can_move_to(entity: &Rc<RefCell<EntityState>>, x: i32, y: i32) -> bool {
        GameState::can_move_towards_point(entity, x as f32, y as f32, 0.6)
    }

    pub fn move_to(entity: &Rc<RefCell<EntityState>>, x: i32, y: i32) -> bool {
        GameState::move_towards_point(entity, x as f32, y as f32, 0.6, None)
    }

    pub fn move_towards_point(entity: &Rc<RefCell<EntityState>>,
                              x: f32, y: f32, dist: f32, cb: Option<Box<ScriptCallback>>) -> bool {
        let anim = STATE.with(|s| {
            let mut state = s.borrow_mut();
            let state = state.as_mut().unwrap();
            debug!("Moving '{}' to {},{}", entity.borrow().actor.actor.name, x, y);

            let start_time = time::Instant::now();
            let path = {
                let area_state = state.area_state.borrow();
                match state.path_finder.find(&area_state, entity.borrow(), x, y, dist) {
                    None => return None,
                    Some(path) => path,
                }
            };
            debug!("Path finding complete in {} secs",
                  util::format_elapsed_secs(start_time.elapsed()));

            let entity = Rc::clone(entity);
            let mut anim = MoveAnimation::new(entity, path, CONFIG.display.animation_base_time_millis);
            anim.set_callback(cb);
            Some(anim)
        });

        match anim {
            None => false,
            Some(anim) => {
                ANIMATIONS.with(|a| {
                    let mut anims = a.borrow_mut();
                    for anim in anims.iter_mut() {
                        anim.check(entity);
                    }
                    anims.push(Box::new(anim));
                });
                true
            }
        }
    }

    pub fn can_move_towards_point(entity: &Rc<RefCell<EntityState>>, x: f32, y: f32, dist: f32) -> bool {
        if entity.borrow().actor.ap() < Module::rules().movement_ap {
            return false;
        }

        STATE.with(|s| {
            let mut state = s.borrow_mut();
            let state = state.as_mut().unwrap();
            let area_state = state.area_state.borrow();

            let start_time = time::Instant::now();
            let val = match state.path_finder.find(&area_state, entity.borrow(), x, y, dist) {
                None => false,
                Some(_) => true,
            };
            debug!("Path finding complete in {} secs",
                  util::format_elapsed_secs(start_time.elapsed()));

            val
        })
    }
}
