
use std::time::Duration;
use bevy::{prelude::*, utils::hashbrown::HashMap};

pub struct PathPlugin;

impl Plugin for PathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (tick_path_timer, update_entity_position))
            .insert_resource(PathTimer::default());
    }
}

pub struct PathDebugPlugin;

impl Plugin for PathDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, debug_render_paths);
    }
}


const MAX_RECURSION_DEPTH: usize = 10;


fn is_any_point_in_triangle(p1: &Vec2, p2: &Vec2, p3: &Vec2, puncture_points: &[PuncturePoint]) -> bool {
    puncture_points
        .iter()
        .any(|puncture_point| puncture_point.is_in_triangle(p1, p2, p3))
}


#[derive(Resource)]
pub struct PathTimer {
    pub timer: Timer,
}


impl Default for PathTimer {
    fn default() -> Self {
        Self {
            timer: Timer::new(Duration::from_millis(250), TimerMode::Repeating),
        }
    }
}

fn tick_path_timer(mut path_timer: ResMut<PathTimer>, time: Res<Time>) {
    path_timer.timer.tick(time.delta());
}

fn update_entity_position(
    mut path_query: Query<(&mut PathType, &Transform)>,
    // path_timer: Res<PathTimer>,
) {
    // if path_timer.timer.just_finished() {
    for (mut path_type, transform) in path_query.iter_mut() {
        let current_position = transform.translation.truncate();
        path_type.push(&current_position);
    }
    // }
}





#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct PuncturePoint {
    position: Vec2,
    name: char,
}



impl PuncturePoint {
    pub const fn new(position: Vec2, name: char) -> Self {
        Self { position, name }
    }
    pub const fn position(&self) -> &Vec2 { &self.position }


    pub fn is_in_triangle(&self, p1: &Vec2, p2: &Vec2, p3: &Vec2) -> bool {
        let p = self.position();
        let denom = (p2.y - p3.y).mul_add(p1.x - p3.x, (p3.x-p2.x) * (p1.y-p3.y));
        if denom.abs() <= f32::EPSILON { return false; }
        let a = (p2.y - p3.y).mul_add(p.x - p3.x, (p3.x-p2.x) * (p.y-p3.y)) / denom;
        let b = (p3.y - p1.y).mul_add(p.x - p3.x, (p1.x-p3.x) * (p.y-p3.y)) / denom;
        let c = 1.0 - a - b;
        [a, b, c].iter().all(|x| (-f32::EPSILON..(1.+f32::EPSILON)).contains(x))
        
    }

    pub fn is_between(&self, p1: &Vec2, p2: &Vec2) -> bool {
        let (y_max, y_min) = (p1.y.max(p2.y), p1.y.min(p2.y));
        if p1.x != p2.x {
            let slope = (p2.y - p1.y) / (p2.x - p1.x);
            if self.position.x != p1.x {
                let self_slope = (self.position.y - p1.y) / (self.position.x - p1.x);
                (slope - self_slope).abs() < f32::EPSILON && self.position.y < y_max && self.position.y > y_min
            } else { 
                false 
            }
        } else {
            (self.position.x - p1.x).abs() < f32::EPSILON && self.position.y < y_max && self.position.y > y_min
        }
    }

    pub fn is_close(&self, p1: &Vec2, p2: &Vec2) -> bool {
        let (y_max, y_min) = (p1.y.max(p2.y), p1.y.min(p2.y));
        if p1.x != p2.x {
            let slope = (p2.y - p1.y) / (p2.x - p1.x);
            if self.position.x != p1.x {
                let self_slope = (self.position.y - p1.y) / (self.position.x - p1.x);
                (slope - self_slope).abs() < f32::EPSILON && self.position.y < y_max && self.position.y > y_min
            } else { 
                false 
            }
        } else {
            (self.position.x - p1.x).abs() < f32::EPSILON && self.position.y < y_max && self.position.y > y_min
        }
    }

    /// positive output means ccw
    /// negative output means cw
    fn winding_update(&self, start: &Vec2, end: &Vec2) -> Option<i8> {
        let start_to_point = self.position - *start;
        let segment_vector = *end - *start;
        let cross_product = start_to_point.x.mul_add(segment_vector.y, -(start_to_point.y * segment_vector.x));
        if cross_product != 0. {
            // let fst = (self.position.y - start.y) * (end.x - start.x);
            // let snd = (end.y - start.y) * (self.position.x - start.x);
            let (smaller, larger) = (start.x.min(end.x), start.x.max(end.x));
            if cross_product < 0. && (smaller..=larger).contains(&self.position().x) { // the segment vector is right rot from the start -> puncture vector.
                return Some(-1);
            }
            if cross_product > 0. && (smaller..=larger).contains(&self.position().x) { // the segment vector is left rot from the start -> puncture vector.
                return Some(1);
            }
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct PLPath {
    nodes: Vec<Vec2>,
}

impl PLPath {
    pub fn start(&self) -> &Vec2 { self.nodes.first().expect("Couldn't get the start point") }
    pub fn end(&self) -> &Vec2 { self.nodes.last().expect("Couldn't get the end point") }
    pub fn get(&self, index: usize) -> &Vec2 { &self.nodes[index] }
    pub fn push(&mut self, position: &Vec2) {
        self.nodes.push(*position);
    }
    pub fn push_transform(&mut self, transform: Transform) {
        self.nodes.push(transform.translation.truncate());
    }
    
    pub fn new(nodes: impl Into<Vec<Vec2>>) -> Self { Self { nodes: nodes.into() } }

    pub fn line(start: Vec2, end: Vec2) -> Self {
        Self {
            nodes: vec![start, end], // straight line corresponds to having no intermediary nodes.
        }
    }

    /// Generates a 
    pub fn auto(start: Vec2, end: Vec2, puncture_points: &[PuncturePoint]) -> Self {
        let mut path = Self::line(start, end);
        let mut depth = 0;
        loop {
            if !path.has_collision(puncture_points) || depth > MAX_RECURSION_DEPTH {
                break;
            }
            path.subdivide_and_shift(puncture_points);
            depth += 1;
        }
        path
    }

    fn has_collision(&self, puncture_points: &[PuncturePoint]) -> bool {
        let size = self.nodes.len();
        if size > 1 {
            (0..size-1).any(
                |i| puncture_points.iter().any(
                    |p| p.is_between(self.get(i), self.get(i+1))
                )
            )
        } else {
            panic!("Not enough elements in list.")
        }
    }

    fn subdivide_and_shift(&mut self, puncture_points: &[PuncturePoint]) {  
        let (start, end) = (*self.start(), *self.end());
        let direction = end - start;
        let normal = Vec2::new(-direction.y, direction.x).normalize(); // Calculate unit normal vector
    
        let offset = (end - start).length() / 2.0;
        let mid = start + direction * (offset / direction.length());
        let nudge_amount = 0.25; // Adjust this value to control the nudge distance
        let nudged_mid = mid + normal * nudge_amount; // Nudge the midpoint by a small amount
    
        self.nodes.insert(0, nudged_mid);
    
        let mut path1 = Self::line(start, nudged_mid);
        let mut path2 = Self::line(nudged_mid, end);
    
        path1.remove_redundant_nodes(puncture_points);
        path2.remove_redundant_nodes(puncture_points);
    
        self.nodes = path1.nodes;
        self.nodes.extend_from_slice(&path2.nodes);
        self.remove_redundant_nodes(puncture_points);
    }

    fn remove_redundant_nodes(&mut self, puncture_points: &[PuncturePoint]) {
        let mut i = 1;
        while i + 2 < self.nodes.len() {
            
            let p1 = self.get(i-1);
            let p2 = self.get(i);
            let p3 = self.get(i+1);
    
            if !is_any_point_in_triangle(p1, p2, p3, puncture_points) {
                self.nodes.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn reverse(&self) -> Self {
        let mut reversed_nodes = self.nodes.clone();
        reversed_nodes.reverse();
        Self { nodes: reversed_nodes }
    }

    pub fn concatenate(&self, other: &Self) -> Self {
        let mut nodes = self.nodes.clone();
        nodes.extend(other.nodes.clone());
        Self { nodes }
    }

    pub fn to_boxed_polyline2d(&self) -> BoxedPolyline2d {
        BoxedPolyline2d::from_iter(self.nodes.clone())
    }

    pub fn to_polyline(&self) -> Polyline2d<128> {
        Polyline2d::new(self.nodes.iter().cloned())
    }
}

#[derive(Debug, Clone, Component)]
pub struct PathType {
    base_path: PLPath,
    current_path: PLPath,
    puncture_points: Vec<PuncturePoint>,
}

impl PathType {
    pub fn new(
        start: Vec2, 
        end: Vec2,
        puncture_points: Vec<PuncturePoint>
    ) -> Self {
        let base_path = PLPath::auto(start, end, &puncture_points);
        let current_path = PLPath::new(vec![start]);

        Self {
            base_path,
            current_path,
            puncture_points,
        }
    }

    pub fn from_path(
        path: PLPath,
        puncture_points: Vec<PuncturePoint>,
    ) -> Self {
        let base_path = PLPath::auto(
            *path.nodes.first().expect("Path is empty!"), 
            *path.nodes.last().expect("Path must be at least length 2!"), 
            &puncture_points
        );
        
        Self {
            base_path,
            current_path: path,
            puncture_points,
        }
    }

    #[must_use]
    pub fn concatenate(&self, other: &PLPath) -> Self {
        Self {
            base_path: self.base_path.clone(),
            current_path: self.current_path.concatenate(other),
            puncture_points: self.puncture_points.clone(),
        }
    }

    pub fn reverse(&mut self) {
        self.current_path = self.current_path.reverse();
    }

    pub fn pop(&mut self) {
        self.current_path.nodes.pop();
    }

    pub fn push(&mut self, point: &Vec2) {
        if self.current_path.nodes.len() > 2 {
            let i = self.current_path.nodes.len() - 2;
            let p1 = &self.current_path.nodes[i];
            let p2 = &self.current_path.nodes[i+1];
            // check if the new node makes the prior node redundant
            if !is_any_point_in_triangle(p1, p2, point, &self.puncture_points) {
                // if so, pop the old node before adding the new one. This saves space.
                self.pop();
                // If the previous node was redundant, maybe the one before that was as well.
                // Here, we have a recursive call to go back and check if the node prior is redundant.
                self.push(point);
            } else {
                // if the prior node was not redundant, then we just push the new one.
                self.current_path.push(point);
            }
        } else {
            // if there's two nodes or fewer, we just push the new node.
            self.current_path.push(point);
        }
    }

    // pub fn word(&self) -> String {
    //     let mut word = String::new();
    //     let mut full_loop = self.current_path.nodes.clone();
    //     full_loop.push(*self.current_path.start());
    //     let mut point_vals = HashMap::<char, i8>::new();
    //     for puncture in &self.puncture_points {
    //         point_vals.insert(puncture.name, 0);
    //     }
    //     for segment in full_loop.windows(2) {
    //         let start = segment[0];
    //         let end = segment[1];
    //         let mut to_append = String::new();
    
    //         // iterate through puncture points.
    //         // Check for first hit of cw or ccw, then check for second hit.
    //         for puncture in &self.puncture_points {
    //             if let Some(n) = puncture.winding_update(&start, &end) {
    //                 let name = puncture.name;
    //                 let entry = point_vals.entry(name).or_insert(0);
    //                 *entry += n;
    //                 if *entry == 2 {
    //                     to_append.push(name.to_ascii_lowercase());
    //                     *entry = 0;
    //                 } else if *entry == -2 {
    //                     to_append.push(name.to_ascii_uppercase());
    //                     *entry = 0;
    //                 }
    //             }
    //         }
    //         word += &to_append;
    //     }
    //     simplify_word(&mut word);
    //     word
    // }

    // pub fn word(&self) -> String {
    //     let mut word = String::new();
    //     let mut full_loop = self.current_path.nodes.clone();
    //     full_loop.push(*self.current_path.start());
    //     let mut point_vals = HashMap::<char, i8>::new();
    //     for puncture in &self.puncture_points {
    //         point_vals.insert(puncture.name, 0);
    //     }
    //     for segment in full_loop.windows(2) {
    //         let start = segment[0];
    //         let end = segment[1];
    //         if start.x < end.x {
    //             for puncture in &self.puncture_points {
    //                 if let Some(n) = puncture.winding_update(&start, &end) {
    //                     let name = puncture.name;
    //                     let entry = point_vals.entry(name).or_insert(0);
    //                     *entry += n;
    //                     if *entry == 2 {
    //                         word.push(name.to_ascii_lowercase());
    //                         *entry = 0;
    //                     } else if *entry == -2 {
    //                         word.push(name.to_ascii_uppercase());
    //                         *entry = 0;
    //                     }
    //                 }
    //             }
    //         } else if start.x > end.x {
    //             for puncture in self.puncture_points.iter().rev() {
    //                 if let Some(n) = puncture.winding_update(&start, &end) {
    //                     let name = puncture.name;
    //                     let entry = point_vals.entry(name).or_insert(0);
    //                     *entry += n;
    //                     if *entry == 2 {
    //                         word.push(name.to_ascii_lowercase());
    //                         *entry = 0;
    //                     } else if *entry == -2 {
    //                         word.push(name.to_ascii_uppercase());
    //                         *entry = 0;
    //                     }
    //                 }
    //             }
    //         }
    //     }
    //     simplify_word(&mut word);
    //     word
    // }

    pub fn word(&self) -> String {
        let mut word = String::new();
        let mut full_loop = self.current_path.nodes.clone();
        full_loop.push(*self.current_path.start());
        let mut point_vals = HashMap::<char, i8>::new();
        let mut first_encounter_order = Vec::<char>::new(); // New vector to track the order of first encounters
        for puncture in &self.puncture_points {
            point_vals.insert(puncture.name, 0);
        }
        for segment in full_loop.windows(2) {
            let start = segment[0];
            let end = segment[1];
            for puncture in &self.puncture_points {
                if let Some(n) = puncture.winding_update(&start, &end) {
                    let name = puncture.name;
                    let entry = point_vals.entry(name).or_insert(0);
                    *entry += n;
                    // If this is the first encounter with this puncture point
                    if *entry == 1 || *entry == -1 {
                        first_encounter_order.push(name); // Add it to the order vector
                    }
                    if *entry % 2 == 0 && entry.signum() == n.signum() {
                        word.push(name.to_ascii_lowercase());
                    }
                }
            }
        }
        // Reorder the word based on the first encounter order
        let mut ordered_word = String::new();
        for name in first_encounter_order {
            ordered_word.push_str(&word.chars().filter(|c| c.to_ascii_uppercase() == name || c.to_ascii_lowercase() == name).collect::<String>());
        }
        simplify_word(&mut ordered_word);
        ordered_word
    }
}


fn simplify_word(word: &mut String) {
    let mut i = 0;
    while i + 1 < word.len() {
        let a = word.as_bytes()[i] as char;
        let b = word.as_bytes()[i + 1] as char;

        if a.to_ascii_uppercase() == b.to_ascii_uppercase() && a != b {
            word.drain(i..i+2);
            i = i.saturating_sub(1);
        } else {
            i += 1;
        }
    }
}



pub fn debug_render_paths(
    path_types: Query<&PathType>,
    mut gizmos: Gizmos
) {
    for path_type in path_types.iter() {
        let polyline = path_type.current_path.to_polyline();
        gizmos.primitive_2d(polyline, Vec2::ZERO, 0.0, Color::WHITE);
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_point_in_triangle() {
        let p1 = &Vec2 { x: 0.0, y: 0.0 };
        let p2 = &Vec2 { x: 4.0, y: 0.0 };
        let p3 = &Vec2 { x: 2.0, y: 4.0 };

        let puncture_point_inside = PuncturePoint { position: Vec2 { x: 2.0, y: 2.0 }, name: 'A' };
        let puncture_point_inside_2 = PuncturePoint { position: Vec2::new(0.1, 0.1), name: 'B' };
        let puncture_point_outside = PuncturePoint { position: Vec2 { x: 5.0, y: 5.0 }, name: 'C' };

        assert!(puncture_point_inside.is_in_triangle(p1, p2, p3));
        assert!(puncture_point_inside_2.is_in_triangle(p1, p2, p3));
        assert!(!puncture_point_outside.is_in_triangle(p1, p2, p3));
    }

    #[test]
    fn test_auto_path() {
        let start = Vec2 { x: 0.0, y: 0.0 };
        let end = Vec2 { x: 4.0, y: 4.0 };
        let puncture_points = [
            PuncturePoint { position: Vec2 { x: 1.0, y: 1.0 }, name: 'A' },
            PuncturePoint { position: Vec2 { x: 2.0, y: 2.0 }, name: 'B' },
            PuncturePoint { position: Vec2 { x: 3.0, y: 3.0 }, name: 'C' },
        ];
        let path = PLPath::auto(start, end, &puncture_points);
        println!("{:?}", path);
    }

    #[test]
    fn test_word() {
        let nodes_1 = [
            Vec2::new(0.0, 0.0),
            Vec2::new(3.0, 6.0),
            Vec2::new(7.0, 6.0),
            Vec2::new(4.0, 4.0),
            Vec2::new(3.0, 6.0),
            Vec2::new(7.0, 6.0),
            Vec2::new(10.0, 0.0),
        ];
        let pl_path_1 = PLPath::new(nodes_1);
        let puncture_points_1: Vec<PuncturePoint> = vec![
            PuncturePoint::new(Vec2::new(5.0, 5.0), 'A')
        ];
        let path_type_1 = PathType::from_path(pl_path_1, puncture_points_1);
        let word_1 = path_type_1.word();
        assert_eq!(word_1, "aa");

        let nodes_2 = [
            Vec2::new(0.0, 0.0),
            Vec2::new(7.0, 6.0),
            Vec2::new(3.0, 6.0),
            Vec2::new(4.0, 4.0),
            Vec2::new(7.0, 6.0),
            Vec2::new(3.0, 6.0),
            Vec2::new(10.0, 0.0),
        ];
        let pl_path_2 = PLPath::new(nodes_2);
        let puncture_points_2: Vec<PuncturePoint> = vec![
            PuncturePoint::new(Vec2::new(5.0, 5.0), 'A')
        ];
        let path_type_2 = PathType::from_path(pl_path_2, puncture_points_2);
        let word_2 = path_type_2.word();
        assert_eq!(word_2, "AA");
    }

    #[test]
    fn test_word_non_trivial_wrapping() {
        let nodes = [
            Vec2::new(0.0, 0.0),  
            Vec2::new(2.0, 1.0),
            Vec2::new(3.0, 2.0),
            Vec2::new(1.0, 2.0), 
            Vec2::new(0.5, 1.0),
            Vec2::new(2.0, 0.5),
            Vec2::new(3.0, 1.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(3.0, 0.0),  
            Vec2::new(4.0, 0.0)
        ];

        let puncture_points = vec![
            PuncturePoint::new(Vec2::new(1.5, 1.5), 'A'),
            PuncturePoint::new(Vec2::new(2.5, 0.5), 'B')
        ];

        let pl_path = PLPath::new(nodes);
        let path_type = PathType::from_path(pl_path, puncture_points);
        let word = path_type.word();

        assert_eq!(word, "aBb");
    }   

    #[test]
    fn test_word_two_punctures() {
        let nodes = [
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 4.0),
            Vec2::new(6.0, 4.0),
            Vec2::new(6.0, 2.0),
            Vec2::new(4.0, 2.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(6.0, 0.0),
            Vec2::new(6.0, 2.0),
            Vec2::new(8.0, 2.0),
            Vec2::new(8.0, 0.0),
            Vec2::new(10.0, 0.0),
        ];
        let pl_path = PLPath::new(nodes);

        let puncture_points: Vec<PuncturePoint> = vec![
            PuncturePoint::new(Vec2::new(4.0, 3.0), 'A'),
            PuncturePoint::new(Vec2::new(7.0, 1.0), 'B'),
        ];

        let path_type = PathType::from_path(pl_path, puncture_points);
        let word = path_type.word();

        // The expected word is "ABA" because:
        // - The path goes around 'A' in the counterclockwise direction (once)
        // - The path goes around 'B' in the counterclockwise direction (once)
        // - The path goes around 'A' in the counterclockwise direction (once again)
        assert_eq!(word, "ABA");
    }
}