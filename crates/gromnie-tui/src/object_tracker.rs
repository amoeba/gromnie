use std::collections::HashMap;
use std::time::Instant;

/// Represents a world object (item, player, container, etc.)
#[derive(Debug, Clone)]
pub struct WorldObject {
    pub object_id: u32,
    pub name: String,
    pub object_type: String,

    // Container/Inventory relationships
    pub container_id: Option<u32>,
    pub items_capacity: Option<u32>,
    pub container_capacity: Option<u32>,

    // Item properties
    pub burden: u32,
    pub value: u32,
    pub stack_size: Option<u32>,
    pub max_stack_size: Option<u32>,

    // Quality/condition tracking
    pub properties: HashMap<String, i32>,

    /// Timestamp when this object was last updated (created or modified)
    pub last_updated: Instant,
}

impl WorldObject {
    pub fn new(object_id: u32, name: String, object_type: String) -> Self {
        Self {
            object_id,
            name,
            object_type,
            container_id: None,
            items_capacity: None,
            container_capacity: None,
            burden: 0,
            value: 0,
            stack_size: None,
            max_stack_size: None,
            properties: HashMap::new(),
            last_updated: Instant::now(),
        }
    }

    pub fn is_container(&self) -> bool {
        self.object_type == "CONTAINER"
    }
}

/// Tracks all objects in the game world, synchronized with server state
#[derive(Debug, Clone, Default)]
pub struct ObjectTracker {
    /// All objects by ObjectId
    pub objects: HashMap<u32, WorldObject>,

    /// Container -> child items mapping
    pub container_contents: HashMap<u32, Vec<u32>>,

    /// Player ID reference
    pub player_id: Option<u32>,

    /// Track recently deleted objects with their deletion time and name
    pub recently_deleted: Vec<(u32, String, Instant)>,
}

impl ObjectTracker {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            container_contents: HashMap::new(),
            player_id: None,
            recently_deleted: Vec::new(),
        }
    }

    /// Process ItemCreateObject message
    pub fn handle_item_create(&mut self, obj: WorldObject) {
        let object_id = obj.object_id;
        self.objects.insert(object_id, obj.clone());

        // Track in container_contents if it has a container
        if let Some(cid) = obj.container_id {
            self.container_contents
                .entry(cid)
                .or_default()
                .push(object_id);
        }
    }

    /// Process ItemDeleteObject message
    pub fn handle_item_delete(&mut self, object_id: u32) {
        if let Some(obj) = self.objects.remove(&object_id) {
            // Track recently deleted with timestamp
            self.recently_deleted.push((object_id, obj.name.clone(), Instant::now()));

            // Remove from parent container
            if let Some(cid) = obj.container_id
                && let Some(contents) = self.container_contents.get_mut(&cid)
            {
                contents.retain(|&id| id != object_id);
            }
        }

        // Remove as a container itself
        self.container_contents.remove(&object_id);
    }

    /// Process ItemMovedObject message - move item between containers
    pub fn handle_item_moved(&mut self, object_id: u32, new_container_id: u32) {
        if let Some(obj) = self.objects.get_mut(&object_id) {
            // Remove from old container
            if let Some(old_cid) = obj.container_id
                && let Some(contents) = self.container_contents.get_mut(&old_cid)
            {
                contents.retain(|&id| id != object_id);
            }

            // Add to new container
            obj.container_id = Some(new_container_id);
            obj.last_updated = Instant::now(); // Mark as updated
            self.container_contents
                .entry(new_container_id)
                .or_default()
                .push(object_id);
        }
    }

    /// Process QualitiesPrivateUpdateInt message
    pub fn handle_quality_update(&mut self, object_id: u32, property_name: String, value: i32) {
        if let Some(obj) = self.objects.get_mut(&object_id) {
            // Handle specific properties that we track separately
            if property_name.as_str() == "StackSize" {
                obj.stack_size = Some(value as u32);
            }

            obj.properties.insert(property_name, value);
            obj.last_updated = Instant::now(); // Mark as updated
        }
    }

    /// Process ItemSetState message (generic state update)
    pub fn handle_item_set_state(&mut self, object_id: u32, property_name: String, value: i32) {
        if let Some(obj) = self.objects.get_mut(&object_id) {
            obj.properties.insert(property_name, value);
            obj.last_updated = Instant::now(); // Mark as updated
        }
    }

    /// Get object by ID
    pub fn get_object(&self, object_id: u32) -> Option<&WorldObject> {
        self.objects.get(&object_id)
    }

    /// Get mutable object by ID
    pub fn get_object_mut(&mut self, object_id: u32) -> Option<&mut WorldObject> {
        self.objects.get_mut(&object_id)
    }

    /// Get all contents of a container
    pub fn get_container_contents(&self, container_id: u32) -> Vec<&WorldObject> {
        self.container_contents
            .get(&container_id)
            .map(|ids| ids.iter().filter_map(|id| self.objects.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get direct player containers (items with container_id == player_id and type == CONTAINER)
    pub fn get_player_containers(&self) -> Vec<&WorldObject> {
        if let Some(player_id) = self.player_id {
            self.objects
                .values()
                .filter(|obj| obj.container_id == Some(player_id) && obj.is_container())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get items directly on player (not in sub-containers)
    pub fn get_player_items(&self) -> Vec<&WorldObject> {
        self.get_container_contents(self.player_id.unwrap_or(0))
            .into_iter()
            .filter(|obj| !obj.is_container())
            .collect()
    }

    /// Set the player ID
    pub fn set_player_id(&mut self, player_id: u32) {
        self.player_id = Some(player_id);
    }

    /// Get total number of objects
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Get all container IDs
    pub fn get_all_container_ids(&self) -> Vec<u32> {
        self.objects
            .values()
            .filter(|obj| obj.is_container())
            .map(|obj| obj.object_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_object() {
        let mut tracker = ObjectTracker::new();
        tracker.set_player_id(1000);

        let mut obj = WorldObject::new(2000, "Belt Pouch".to_string(), "CONTAINER".to_string());
        obj.container_id = Some(1000);
        obj.burden = 500;
        obj.value = 195725;
        obj.items_capacity = Some(24);
        tracker.handle_item_create(obj);

        assert_eq!(tracker.object_count(), 1);
        let obj = tracker.get_object(2000).unwrap();
        assert_eq!(obj.name, "Belt Pouch");
        assert!(obj.is_container());
    }

    #[test]
    fn test_container_contents() {
        let mut tracker = ObjectTracker::new();
        tracker.set_player_id(1000);

        // Create a container
        let mut obj = WorldObject::new(2000, "Belt Pouch".to_string(), "CONTAINER".to_string());
        obj.container_id = Some(1000);
        obj.burden = 500;
        obj.value = 195725;
        obj.items_capacity = Some(24);
        tracker.handle_item_create(obj);

        // Create items in the container
        let mut obj = WorldObject::new(3000, "Potion".to_string(), "CONSUMABLE".to_string());
        obj.container_id = Some(2000);
        obj.burden = 10;
        obj.value = 50;
        obj.stack_size = Some(5);
        obj.max_stack_size = Some(100);
        tracker.handle_item_create(obj);

        let contents = tracker.get_container_contents(2000);
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].name, "Potion");
    }

    #[test]
    fn test_move_item() {
        let mut tracker = ObjectTracker::new();
        tracker.set_player_id(1000);

        // Create two containers
        let mut obj = WorldObject::new(2000, "Belt Pouch".to_string(), "CONTAINER".to_string());
        obj.container_id = Some(1000);
        obj.burden = 500;
        obj.value = 195725;
        obj.items_capacity = Some(24);
        tracker.handle_item_create(obj);

        let mut obj = WorldObject::new(2001, "Backpack".to_string(), "CONTAINER".to_string());
        obj.container_id = Some(1000);
        obj.burden = 1000;
        obj.value = 395725;
        obj.items_capacity = Some(50);
        tracker.handle_item_create(obj);

        // Create item in first container
        let mut obj = WorldObject::new(3000, "Potion".to_string(), "CONSUMABLE".to_string());
        obj.container_id = Some(2000);
        obj.burden = 10;
        obj.value = 50;
        tracker.handle_item_create(obj);

        assert_eq!(tracker.get_container_contents(2000).len(), 1);

        // Move item to second container
        tracker.handle_item_moved(3000, 2001);

        assert_eq!(tracker.get_container_contents(2000).len(), 0);
        assert_eq!(tracker.get_container_contents(2001).len(), 1);
    }
}
