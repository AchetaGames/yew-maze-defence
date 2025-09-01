use crate::model::RunState;

// Interactable mask helper (extract from main.rs)
pub fn compute_interactable_mask(rs: &RunState) -> Vec<bool> {
    use std::collections::VecDeque;
    let gs = rs.grid_size;
    let n = rs.tiles.len();
    let mut mask = vec![false; n];
    let mut reachable = vec![false; n];
    let idx = |x: u32, y: u32| (y * gs.width + x) as usize;
    let inb = |x: i32, y: i32| x >= 0 && y >= 0 && (x as u32) < gs.width && (y as u32) < gs.height;
    let mut q: VecDeque<(u32, u32)> = VecDeque::new();
    let push = |x: u32, y: u32, reach: &mut Vec<bool>, q: &mut VecDeque<(u32, u32)>| {
        let i = idx(x, y);
        if !reach[i] {
            reach[i] = true;
            q.push_back((x, y));
        }
    };
    let seeds: Vec<crate::model::Position> = if !rs.path_loop.is_empty() {
        rs.path_loop.clone()
    } else {
        rs.path.clone()
    };
    for p in &seeds {
        if p.x < gs.width && p.y < gs.height {
            let i = idx(p.x, p.y);
            if matches!(
                rs.tiles[i].kind,
                crate::model::TileKind::Empty
                    | crate::model::TileKind::Start
                    | crate::model::TileKind::Direction { .. }
            ) {
                push(p.x, p.y, &mut reachable, &mut q);
            }
        }
    }
    if q.is_empty() {
        for (i, t) in rs.tiles.iter().enumerate() {
            if matches!(t.kind, crate::model::TileKind::Start) {
                let x = (i as u32) % gs.width;
                let y = (i as u32) / gs.width;
                push(x, y, &mut reachable, &mut q);
                break;
            }
        }
    }
    let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    while let Some((x, y)) = q.pop_front() {
        let i = idx(x, y);
        mask[i] = true;
        for (dx, dy) in dirs {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if !inb(nx, ny) {
                continue;
            }
            let ux = nx as u32;
            let uy = ny as u32;
            let ni = idx(ux, uy);
            match rs.tiles[ni].kind {
                crate::model::TileKind::Empty
                | crate::model::TileKind::Start
                | crate::model::TileKind::Direction { .. } => {
                    if !reachable[ni] {
                        reachable[ni] = true;
                        q.push_back((ux, uy));
                    }
                }
                _ => {}
            }
        }
    }
    for y in 0..gs.height {
        for x in 0..gs.width {
            let i = idx(x, y);
            match rs.tiles[i].kind {
                crate::model::TileKind::Rock { .. } | crate::model::TileKind::Wall => {
                    let mut adj = false;
                    for (dx, dy) in dirs {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if inb(nx, ny) {
                            let ni = idx(nx as u32, ny as u32);
                            if reachable[ni] {
                                adj = true;
                                break;
                            }
                        }
                    }
                    if adj {
                        mask[i] = true;
                    }
                }
                _ => {}
            }
        }
    }
    mask
}
