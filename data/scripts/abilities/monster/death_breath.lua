max_dist = 12

function on_activate(parent, ability)
  local targets = parent:targets():without_self()
  
  local min_dist = math.max(parent:width(), parent:height()) / 2.0
  
  local targeter = parent:create_targeter(ability)
  targeter:set_selection_radius(max_dist)
  targeter:set_free_select(max_dist * 2)
  targeter:set_shape_cone(parent:center_x(), parent:center_y(), min_dist, max_dist, math.pi / 3) 
  targeter:add_all_effectable(targets)
  targeter:invis_blocks_affected_points(true)
  targeter:activate()
end

function on_target_select(parent, ability, targets)
  local pos = targets:selected_point()
  
  local delta_x = pos.x - parent:x()
  local delta_y = pos.y - parent:y()
  local angle = game:atan2(delta_x, delta_y)
  
  local duration = 1.5
  
  local gen = parent:create_particle_generator("fire_particle", duration)
  gen:set_position(gen:param(parent:center_x() + 2.0), gen:param(parent:center_y() - 0.5))
  gen:set_color(gen:param(0.5), gen:param(0.0), gen:param(1.0))
  gen:set_gen_rate(gen:param(500.0, -500))
  gen:set_initial_gen(500.0)
  gen:set_particle_size_dist(gen:fixed_dist(0.7), gen:fixed_dist(0.7))
  gen:set_particle_position_dist(
    gen:dist_param(gen:uniform_dist(-0.1, 0.1),
    gen:angular_dist(angle - math.pi / 6, angle + math.pi / 6, 0, 20)))
    
  gen:set_particle_duration_dist(gen:fixed_dist(0.6))
  
  local targets_table = targets:to_table()
  for i = 1, #targets_table do
    local dist = parent:dist_to_entity(targets_table[i])
    local cb_dur = duration * dist / max_dist
    
    local cb = ability:create_callback(parent)
	cb:add_target(targets_table[i])
	cb:set_on_anim_update_fn("attack_target")
    gen:add_callback(cb, cb_dur)
  end
  
  gen:activate()
  ability:activate(parent)
end

function attack_target(parent, ability, targets)
  local target = targets:first()

  if target:is_valid() then
    local stats = parent:stats()
	local min_dmg = 10 + stats.caster_level / 3 + stats.intellect_bonus / 6
    local max_dmg = 20 + stats.intellect_bonus / 3 + stats.caster_level * 0.667
    parent:special_attack(target, "Reflex", "Ranged", min_dmg, max_dmg, 0, "Raw")
  end
end

