use std::{cmp::Ordering, sync::Arc, time::Duration};
use bevy::prelude::*;


/// Primary plugin for 
/// Adds systems for updating the path timer and updating the position of entities along the path.
pub struct PathPlugin;

impl Plugin for PathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (tick_path_timer, update_entity_position))
            .insert_resource(PathTimer::default());
    }
}

/// Plugin for debugging paths.
/// Adds a system for rendering paths to the screen using Bevy's 2d primitives.
pub struct PathDebugPlugin;

impl Plugin for PathDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, debug_render_paths);
    }
}



/// Checks if a triangle defined by three points should not be removed based on a list of puncture points.
fn should_not_remove(p1: &Vec2, p2: &Vec2, p3: &Vec2, puncture_points: &[PuncturePoint]) -> bool {
    puncture_points
        .iter()
        .any(|p| p.should_not_remove(p1, p2, p3))
}

/// Resource struct representing a timer for path updates.
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

/// Updates the path timer.
fn tick_path_timer(mut path_timer: ResMut<PathTimer>, time: Res<Time>) {
    path_timer.timer.tick(time.delta());
}

/// Updates the position of entities along the path.
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

/// `PuncturePoint` represents a hole in the plane from the perspective of homotopy.
/// 
/// A `PuncturePoint` is a point in the plane that acts as a puncture or hole, affecting the homotopy type
/// of paths traveling around it.
///
/// Each `PuncturePoint` contains a `position` which is a `Vec2` representing the position of the point,
/// and a `name` which is a `char` that uniquely identifies the puncture point. It is used to represent
/// the traversal around the puncture point when writing the homotopy type of a path. `name.to_ascii_lowercase()`
/// is used to represent clockwise traversal around the puncture point, and `name.to_ascii_uppercase()` is used 
/// to represent counterclockwise traversal.
///
/// Note that the name character is made uppercase upon instantiation.
/// 
/// # Examples
///
/// ```
/// use bevy::prelude::*;
/// use charred_path::piecewise_linear::PuncturePoint;
/// 
/// let position = Vec2::new(1.0, 2.0);
/// let puncture_point = PuncturePoint::new(position, 'a');
/// assert_eq!(puncture_point.position(), &position);
/// assert_eq!(puncture_point.name(), 'A');
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct PuncturePoint {
    position: Vec2,
    name: char,
}


impl PuncturePoint {
    /// Represents a puncture point in the plane.
    pub const fn new(position: Vec2, name: char) -> Self {
        Self { position, name: name.to_ascii_uppercase() }
    }

    /// Returns the position of the puncture point in 2D.
    pub const fn position(&self) -> &Vec2 { &self.position }

    /// Returns the label associated to the puncture point.
    pub const fn name(&self) -> char { self.name }

    /// Checks if the puncture point is inside a triangle defined by three points.
    fn is_in_triangle(&self, p1: &Vec2, p2: &Vec2, p3: &Vec2) -> bool {
        let p = self.position();
        let denom = (p2.y - p3.y).mul_add(p1.x - p3.x, (p3.x-p2.x) * (p1.y-p3.y));
        if denom.abs() <= f32::EPSILON { return false; }
        let a = (p2.y - p3.y).mul_add(p.x - p3.x, (p3.x-p2.x) * (p.y-p3.y)) / denom;
        let b = (p3.y - p1.y).mul_add(p.x - p3.x, (p1.x-p3.x) * (p.y-p3.y)) / denom;
        let c = 1.0 - a - b;
        [a, b, c].iter().all(|x| (-f32::EPSILON..(1.+f32::EPSILON)).contains(x))
        
    }

    /// Checks if the puncture point should not be removed based on its position relative to a triangle.
    fn should_not_remove(&self, p1: &Vec2, p2: &Vec2, p3: &Vec2) -> bool {
        let x = self.position().x;
        self.is_in_triangle(p1, p2, p3) 
            || ((p1.x..p2.x).contains(&x) && p2.x < p3.x && (x-p2.x).abs() > 1e-4)
            || ((p2.x..p1.x).contains(&x) && p3.x < p2.x && (x-p2.x).abs() > 1e-4)
    }

    /// Updates the winding of the puncture point based on its position relative to a line segment.
    ///
    /// # Arguments
    ///
    /// * `start` - The starting point of the line segment.
    /// * `end` - The ending point of the line segment.
    ///
    /// # Returns
    ///
    /// Returns `Some(1)` if the line passes left -> right above the point, 
    /// `Some(-1)` if the line passes right -> left above the point, and
    /// `None` otherwise.
    fn winding_update(&self, start: &Vec2, end: &Vec2) -> Option<i32> {
        let position = self.position();
        let cross_product = (end.y - start.y).mul_add(position.x - start.x, -((position.y - start.y) * (end.x - start.x)));
        // Check if position is below the line segment
        if cross_product > 0. && (start.x..end.x).contains(&position.x) {
            return Some(1);
        }
        if cross_product < 0. && (end.x..start.x).contains(&position.x) {
            return Some(-1);
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct PLPath {
    nodes: Vec<Vec2>,
}

impl PLPath {
    /// Gets the first node, if there is one.
    /// 
    /// ## Panics
    /// This will panic if `nodes` is empty.
    fn start(&self) -> &Vec2 { self.nodes.first().expect("Couldn't get the start point") }

    /// Gets the last node, if there is one.
    /// 
    /// ## Panics
    /// This will panic if `nodes` is empty.
    fn end(&self) -> &Vec2 { self.nodes.last().expect("Couldn't get the end point") }

    /// 
    fn push(&mut self, position: &Vec2) {
        self.nodes.push(*position);
    }
    /// Appends the XY-position of a Transform 
    pub fn push_transform(&mut self, transform: Transform) {
        self.nodes.push(transform.translation.truncate());
    }
    
    /// A new path from a list of nodes.
    pub fn new(nodes: impl Into<Vec<Vec2>>) -> Self { Self { nodes: nodes.into() } }

    /// A straight line path from start to end.
    pub fn line(start: Vec2, end: Vec2) -> Self {
        Self {
            nodes: vec![start, end],
        }
    }

    /// Path whose nodes are reversed from `self.nodes`.
    pub fn reverse(&self) -> Self {
        let mut reversed_nodes = self.nodes.clone();
        reversed_nodes.reverse();
        Self { nodes: reversed_nodes }
    }

    /// Returns a PLPath whose nodes are `self.nodes` concatenated by `other.nodes`.
    pub fn concatenate(&self, other: &Self) -> Self {
        let mut nodes = self.nodes.clone();
        nodes.extend(other.nodes.clone());
        Self { nodes }
    }

    /// An iterable containing each linear component of the path as a Segment2d.
    /// Used to display the PL path as a loop for debugging purposes.
    fn to_segment2d_iter(&self) -> impl Iterator<Item = (Segment2d, Vec2)> + '_ {
        let last = if self.start() != self.end() {
            Some(Segment2d::from_points(*self.end(), *self.start()))
        } else {
            None
        };
        self.nodes.windows(2).filter_map(|pair| {
            let point1 = pair[0];
            let point2 = pair[1];
            if point1 == point2 {
                None
            } else {
                let segment = Segment2d::from_points(point1, point2);
                Some(segment)
            }
        }).chain(last)
    }
}


#[derive(Debug, Clone, Component)]
pub struct PathType {
    current_path: PLPath,
    puncture_points: Arc<[PuncturePoint]>,
}

impl PathType {
    pub fn new(
        start: Vec2, 
        puncture_points: Vec<PuncturePoint>
    ) -> Self {
        Self {
            current_path: PLPath::new(vec![start]),
            puncture_points: puncture_points.into(),
        }
    }

    pub fn from_path(
        path: PLPath,
        puncture_points: Vec<PuncturePoint>,
    ) -> Self {
        Self {
            current_path: path,
            puncture_points: puncture_points.into(),
        }
    }

    #[must_use]
    pub fn concatenate(&self, other: &PLPath) -> Self {
        Self {
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

    /// Appends a 2d position to the end of the current path.
    pub fn push(&mut self, point: &Vec2) {
        let _ = &self.current_path.nodes.split_last();
        if let [.., p1, p2] = &self.current_path.nodes[..] {
            if !should_not_remove(p1, p2, point, &self.puncture_points) {
                self.pop();
                self.push(point);
            } else {
                self.current_path.push(point); 
            }
        } else {
            self.current_path.push(point);
        }
    }

    pub fn word(&self) -> String {
        let mut word = String::new();
        let full_loop: Vec<&Vec2> = self.current_path.nodes
            .iter()
            .chain(std::iter::once(self.current_path.start()))
            .collect();
        for segment in full_loop.windows(2) {
            let (start, end) = (segment[0], segment[1]);
            let punctures: Vec<&PuncturePoint> = match start.x.partial_cmp(&end.x) {
                Some(Ordering::Less) 
                    => self.puncture_points
                        .iter()
                        .collect(),
                Some(Ordering::Greater) 
                    => self.puncture_points
                        .iter()
                        //.rev()
                        .collect(),
                _ => continue,
            };
            for puncture in punctures {
                if let Some(n) = puncture.winding_update(start, end) {
                    match n {
                        1 => word.push(puncture.name.to_ascii_lowercase()),
                        -1 => word.push(puncture.name.to_ascii_uppercase()),
                        _ => {}
                    }
                }
            }
        }

        simplify_word(&mut word);
        word
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

/// # Rendering PL Paths for Debug purposes
/// This visualizes the piecewise-linear paths.
fn debug_render_paths(
    path_types: Query<&PathType>,
    mut gizmos: Gizmos,
) {
    for path_type in path_types.iter() {
        if path_type.current_path.nodes.len() > 1 {
            for segment in path_type.current_path.to_segment2d_iter() {
                gizmos.primitive_2d(segment.0, segment.1, 0.0, Color::WHITE);
            }
        }
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
}