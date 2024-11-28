#![feature(gen_blocks)]

pub const WIDTH: usize = 5;
pub const HEIGHT: usize = 5;
pub const NEIGHBORS: i32 = 3;

#[derive(Debug,Clone)]
pub struct Board {
    // 0: empty,
    // 1/2: player 1/2 upright,
    // -1/-2: player 1/2 flipped
    cells: [i8;WIDTH*HEIGHT],
    // precomputed score
    score: i32,
}
impl Board {
    pub fn new() -> Self {
        Self { cells: [0; WIDTH*HEIGHT], score: 0, }
    }
    pub fn score(&self) -> i32 {
        let mut score = 0;
        for idx in 0..self.cells.len() {
            let x = idx%WIDTH;
            let y = idx/WIDTH;
            let piece = self.cells[idx];
            let sum = self.neighbor_count(idx,x,y);
            if sum >= NEIGHBORS {
                if piece == -1 { score += 1; }
                if piece == -2 { score -= 1; }
            }
        }
        assert!(score == self.score, "wat {} != {}\n{}", score, self.score, self);
        score
    }
    pub fn neighbor_count(&self, idx: usize, x: usize, y: usize) -> i32 {
        let piece = self.cells[idx].abs();
        let mut sum = 0;
        let w = WIDTH;
        let h = HEIGHT;
        sum += (x > 0   && self.cells[idx-1].abs() == piece) as i32;
        sum += (x < w-1 && self.cells[idx+1].abs() == piece) as i32;
        sum += (y > 0   && self.cells[idx-w].abs() == piece) as i32;
        sum += (y < h-1 && self.cells[idx+w].abs() == piece) as i32;
        sum
    }
    pub fn update(&mut self, mod_idx: usize, idx: usize, x: usize, y: usize, new_place: bool) {
        let piece = self.cells[idx];
        //if piece <= 0 { return; }
        // if this piece isn't the same owner as the added piece,
        // it can't possibly change the score or get flipped
        if self.cells[idx].abs() != self.cells[mod_idx].abs() { return; }
        let sum = self.neighbor_count(idx,x,y);
        if sum >= NEIGHBORS {
            if piece > 0 { self.cells[idx] *= -1; }
            if sum == NEIGHBORS || new_place {
                if piece.abs() == 1 { self.score += 1; }
                if piece.abs() == 2 { self.score -= 1; }
                //println!("score inc, piece={} at {}, score={}", piece, idx, self.score);
            }
        }
    }
    pub fn propagate(&mut self, idx: usize) {
        let x = idx%WIDTH;
        let y = idx/WIDTH;
        let w = WIDTH;
        let h = HEIGHT;
        self.update(idx,idx, x,y, true);
        if x > 0   { self.update(idx, idx-1,x-1,y, false); }
        if x < w-1 { self.update(idx, idx+1,x+1,y, false); }
        if y > 0   { self.update(idx, idx-w,x,y-1, false); }
        if y < h-1 { self.update(idx, idx+w,x,y+1, false); }
    }
    pub fn update_delete(&mut self, mod_idx: usize, idx: usize, x: usize, y: usize) {
        // if this piece isn't the same owner as the removed piece,
        // it can't possibly decrease the score
        if self.cells[idx].abs() != self.cells[mod_idx].abs() { return; }
        let owner = self.cells[idx].abs();
        let sum = self.neighbor_count(idx, x, y);
        if sum == NEIGHBORS {
            // this piece is exactly "borderline", so the removal will make it non-scoring, losing
            // a point
            if owner == 1 { self.score -= 1; }
            if owner == 2 { self.score += 1; }
            //println!("score decrease, piece at {} score={}", idx, self.score);
        }
    }
    // inform the score counter that a swap moved a piece that might possibly affect the score
    pub fn propagate_delete(&mut self, idx: usize) {
        // precondition: self.cells[idx] == 1 or 2
        assert!(self.cells[idx] > 0);
        let x = idx%WIDTH;
        let y = idx/WIDTH;
        let w = WIDTH;
        let h = HEIGHT;
        if x > 0   { self.update_delete(idx,idx-1,x-1,y); }
        if x < w-1 { self.update_delete(idx,idx+1,x+1,y); }
        if y > 0   { self.update_delete(idx,idx-w,x,y-1); }
        if y < h-1 { self.update_delete(idx,idx+w,x,y+1); }
    }
    pub fn moves(self, player: i8) -> impl Iterator<Item = Board> {
        // return statement because otherwise tree-sitter's indentation thingy gets fucked
        return gen move {
            for c in 0..self.cells.len() {
                if self.cells[c] == 0 {
                    let mut b = self.clone();
                    b.cells[c] = player+1;
                    //println!("FUCK\n{}", b);
                    b.propagate(c);
                    //println!("place {}", c);
                    //b.score();
                    yield b;
                }
            }
            /*let mut swaps = vec![];
            for i in 0..self.cells.len() {
                if self.cells[i] > 0 {
                    swaps.push(i);
                }
            }*/
            for i in 0..self.cells.len() {
                if self.cells[i] <= 0 { continue; }
                for j in i+1..self.cells.len() {
                    if self.cells[j] <= 0 { continue; }
                    let mut b = self.clone();
                    //println!("{},{}:\n{}", i,j,b);
                    b.propagate_delete(i);
                    b.propagate_delete(j);
                    b.cells.swap(i,j);
                    b.cells[i] *= -1;
                    b.cells[j] *= -1;
                    b.propagate(i);
                    b.propagate(j);
                    //println!("swap {},{}", i, j);
                    //b.score();
                    //if b.cells[i] > 0 { b.cells[i] *= -1; }
                    //if b.cells[j] > 0 { b.cells[j] *= -1; }
                    yield b;
                }
            }
        }.into_iter();
    }
    pub fn hash(&self) -> u128 {
        self.cells.iter().fold(0u128,|a,e| {
            (a << 3) | ((e+2) as u128)
        })
    }
}
impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "===")?;
        for (i,c) in self.cells.iter().enumerate() {
            if i != 0 && i % WIDTH == 0 { writeln!(f)?; }
            write!(f, "{}", b"OX.xo"[(c+2) as usize] as char)?;
        }
        //writeln!(f)
        Ok(())
    }
}

#[derive(Default)]
pub struct Solver {
    cache: std::collections::HashMap<u128, ((i32,f32),Option<Board>)>,
}
impl Solver {
    pub fn minimax(&mut self, b: &Board, player: i8, depth: i32) -> ((i32,f32),Option<Board>) {
        let b_hash = b.hash();
        if let Some((i,b)) = self.cache.get(&b_hash) { return (*i,b.clone()); }
        let b_score = b.score;
        if depth >= 5 {
            return ((b_score, b_score as f32), None);
        }
        let f = b.clone().moves(player);
        let mut child_avg = 0.0;
        let mut best_so_far = (0,0.0);
        let mut best_board = None;
        let mut l = 0;
        for (_idx,i) in f.enumerate() {
            l += 1;
            if depth <= 1 {
                //println!("depth={} i={}/{} best={:?}", depth,idx,l, best_so_far);
            }
            let (s,_b) = self.minimax(&i,player^1,depth+1);
            child_avg += s.1;
            let better = if player == 0 {
                s > best_so_far
            } else {
                s < best_so_far
            };
            if best_board.is_none() || better {
                best_so_far = s;
                best_board = Some(i);
            }
            //if (player == 0 && best_so_far > 0) || (player == 1 && best_so_far < 0) {
                //break; 
            //}
        }
        child_avg /= l as f32;
        best_so_far.1 = child_avg;
        if best_board.is_none() { best_so_far.0 = b_score; best_so_far.1 = best_so_far.0 as f32; }
        self.cache.insert(b_hash, (best_so_far, best_board.clone()));
        (best_so_far,best_board)
    }
}


fn main() {
    //use rand::Rng;
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    let mut b = Board::new();
    let mut solver = Solver::default();
    let mut turn = 0;
    loop {
        solver.cache.clear();
        if false && turn&1 == 1 {
            let moves: Vec<_> = b.moves(turn&1).collect();
            if moves.len() == 0 { return; }
            b = moves.choose(&mut rng).unwrap().clone();
        } else {
            let (_s,br) = solver.minimax(&b, turn&1, 0);
            if br.is_none() { return; }
            b = br.unwrap();
        }
        turn += 1;
        println!("{}", b);
        println!("Score {}", b.score);
    }
}
