function on_activate(parent, ability)
  local targets = parent:targets():hostile():visible()
  
  local targeter = parent:create_targeter(ability)
  targeter:set_selection_visible()
  targeter:add_all_selectable(targets)
  targeter:add_all_effectable(targets)
  targeter:activate()
end

function on_target_select(parent, ability, targets)
  local target = targets:first()
  
  local speed = 10.0
  local dist = parent:dist_to_entity(target)
  local duration = dist / speed
  local parent_center_y = parent:center_y() - 1.0
  local vx = (target:center_x() - parent:center_x()) / duration
  local vy = (target:center_y() - parent_center_y) / duration
  
  local cb = ability:create_callback(parent)
  cb:add_target(target)
  cb:set_on_anim_update_fn("attack_target")
  
  local gen = parent:create_particle_generator("fire_particle", duration)
  gen:set_position(gen:param(parent:center_x(), vx), gen:param(parent_center_y, vy))
  gen:set_gen_rate(gen:param(70.0))
  gen:set_initial_gen(35.0)
  gen:set_particle_size_dist(gen:fixed_dist(0.5), gen:fixed_dist(0.5))
  gen:set_particle_position_dist(gen:dist_param(gen:uniform_dist(-0.2, 0.2), gen:uniform_dist(-vx / 5.0, 0.0)),
    gen:dist_param(gen:uniform_dist(-0.2, 0.2), gen:uniform_dist(-vy / 5.0, 0.0)))
  gen:set_particle_duration_dist(gen:fixed_dist(0.6))
  gen:add_callback(cb, duration - 0.1)
  gen:activate()
  
  ability:activate(parent)
end

function attack_target(parent, ability, targets)
  local target = targets:first()
  
  local stats = parent:stats()
  local min_dmg = 18 + stats.caster_level / 2 + stats.intellect_bonus / 4
  local max_dmg = 28 + stats.intellect_bonus / 2 + stats.caster_level
  parent:special_attack(target, "Reflex", "Spell", min_dmg, max_dmg, 0, "Fire")
  
  local gen = target:create_particle_generator("fire_particle", 0.6)
  gen:set_initial_gen(50.0)
  gen:set_position(gen:param(target:center_x()), gen:param(target:center_y()))
  gen:set_particle_size_dist(gen:fixed_dist(0.3), gen:fixed_dist(0.3))
  gen:set_particle_position_dist(gen:dist_param(gen:uniform_dist(-0.2, 0.2), gen:uniform_dist(-5.0, 5.0)),
    gen:dist_param(gen:uniform_dist(-0.2, 0.2), gen:uniform_dist(-5.0, 5.0), gen:fixed_dist(5.0)))
  gen:set_particle_duration_dist(gen:fixed_dist(0.6))
  gen:activate()
end
