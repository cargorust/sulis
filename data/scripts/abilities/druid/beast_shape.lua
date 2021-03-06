held_main_id = "__beast_shape_equip_held_main"
held_off_id = "__beast_shape_equip_held_off"

function on_activate(parent, ability)
  ability:activate(parent)
  
  local effect = parent:create_effect(ability:name(), ability:duration())
  effect:set_tag("shapeshift")
  
  local cb = ability:create_callback(parent)
  cb:set_on_removed_fn("on_removed")
  effect:add_callback(cb)
  
  local attr_bonus = 2
  if parent:ability_level(ability) > 1 then
    attr_bonus = 6
  end
  
  effect:add_attribute_bonus("Strength", attr_bonus)
  effect:add_attribute_bonus("Dexterity", attr_bonus)
  effect:add_attribute_bonus("Endurance", attr_bonus)
  effect:add_abilities_disabled()
  
  local stats = parent:stats()
  local level = stats.caster_level / 2 + stats.wisdom_bonus / 4
 
  effect:add_num_bonus("armor", 10 + level - stats.base_armor)
  effect:add_num_bonus("defense", 50 + level * 2 - stats.defense)
  effect:add_num_bonus("melee_accuracy", 50 + level * 2 - stats.melee_accuracy)
  
  local inv = parent:inventory()
  local held_main = inv:unequip_item("held_main")
  local held_off = inv:unequip_item("held_off")
  
  if held_main:is_valid() then
    parent:set_flag(held_main_id, held_main:id())
  end
  
  if held_off:is_valid() then
    parent:set_flag(held_off_id, held_off:id())
  end
  
  local item = game:add_party_item("werewolf_claw")
  inv:equip_item(item)
  inv:set_locked(true)
  
  local gen = parent:create_image_layer_anim()
  gen:add_image("Ears", "empty")
  gen:add_image("Hair", "empty")
  gen:add_image("Beard", "empty")
  gen:add_image("Head", "empty")
  gen:add_image("Hands", "empty")
  gen:add_image("Foreground", "creatures/werewolf")
  gen:add_image("Torso", "empty")
  gen:add_image("Legs", "empty")
  gen:add_image("Feet", "empty")
  gen:add_image("Background", "empty")
  effect:add_image_layer_anim(gen)
  
  effect:apply()
  
  local anim = parent:create_color_anim(1.0)
  anim:set_color_sec(anim:param(1.0, -1,0),
                     anim:param(1.0, -1,0),
                     anim:param(1.0, -1,0),
                     anim:param(0.0))
  anim:activate()
  
  game:play_sfx("sfx/roar5")
end

function on_removed(parent, ability)
  local inv = parent:inventory()
  inv:set_locked(false)
  local item = inv:unequip_item("held_main")
   
  if item:id() == "werewolf_claw" then
	game:remove_party_item(item)
  end
   
  local anim = parent:create_color_anim(1.0)
  anim:set_color_sec(anim:param(1.0, -1,0),
                     anim:param(1.0, -1,0),
                     anim:param(1.0, -1,0),
                     anim:param(0.0))
  anim:activate()
  
  if parent:has_flag(held_main_id) then
     local item_id = parent:get_flag(held_main_id)
	 local item = game:find_party_item(item_id)
	 if item:is_valid() then
	   inv:equip_item(item)
	 end
   
     parent:clear_flag(held_main_id)
   end
   
   if parent:has_flag(held_off_id) then
     local item_id = parent:get_flag(held_off_id)
	 local item = game:find_party_item(item_id)
	 if item:is_valid() then
	   inv:equip_item(item)
	 end
	 
     parent:clear_flag(held_off_id)
   end
end