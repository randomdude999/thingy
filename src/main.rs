#![feature(gen_blocks)]

use rand::SeedableRng;

pub const WIDTH: usize = 5;
pub const HEIGHT: usize = 5;
pub const NEIGHBORS: i32 = 3;

static mut ZOBRIST: [u64; WIDTH*HEIGHT*4] = [0; WIDTH*HEIGHT*4];
fn get_zobrist(ind: usize, el: u8) -> u64 {
    if el == 0 {
        0
    } else {
        unsafe { ZOBRIST[ind*4 + (el & 3) as usize] }
    }
}

#[derive(Debug,Clone)]
pub struct Board {
    // bit 0: player 1/2
    // bit 1: upright/flipped
    // bit 2: empty/full
    // thus: empty is still value 0
    // "ignore flippedness" is &5
    cells: [u8;WIDTH*HEIGHT],
    // precomputed score
    score: i32,
    // precomp hash
    hash: u64,
}
impl Board {
    pub fn new() -> Self {
        Self { cells: [0; WIDTH*HEIGHT], score: 0, hash: 0, }
    }
    pub fn score(&self) -> i32 {
        let mut score = 0;
        for idx in 0..self.cells.len() {
            let x = idx%WIDTH;
            let y = idx/WIDTH;
            let piece = self.cells[idx];
            let sum = self.neighbor_count(idx,x,y);
            if sum >= NEIGHBORS {
                if piece == 6 { score += 1; }
                if piece == 7 { score -= 1; }
            }
        }
        score
    }
    pub fn neighbor_count(&self, idx: usize, x: usize, y: usize) -> i32 {
        let piece = self.cells[idx] & 5;
        let mut sum = 0;
        let w = WIDTH;
        let h = HEIGHT;
        sum += (x > 0   && self.cells[idx-1] & 5 == piece) as i32;
        sum += (x < w-1 && self.cells[idx+1] & 5 == piece) as i32;
        sum += (y > 0   && self.cells[idx-w] & 5 == piece) as i32;
        sum += (y < h-1 && self.cells[idx+w] & 5 == piece) as i32;
        sum
    }
    pub fn update(&mut self, mod_idx: usize, idx: usize, x: usize, y: usize, new_place: bool) {
        let piece = self.cells[idx];
        // if this piece isn't the same owner as the added piece,
        // it can't possibly change the score or get flipped
        if self.cells[idx] & 5 != self.cells[mod_idx] & 5 { return; }
        let sum = self.neighbor_count(idx,x,y);
        if sum >= NEIGHBORS {
            if piece & 2 == 0 {
                self.cells[idx] |= 2;
                self.hash ^= get_zobrist(idx, piece) ^ get_zobrist(idx, piece | 2);
            }
            if sum == NEIGHBORS || new_place {
                if piece & 1 == 0 { self.score += 1; }
                if piece & 1 == 1 { self.score -= 1; }
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
        if self.cells[idx] & 5 != self.cells[mod_idx] & 5 { return; }
        let owner = self.cells[idx] & 1;
        let sum = self.neighbor_count(idx, x, y);
        if sum == NEIGHBORS {
            // this piece is exactly "borderline", so the removal
            // will make it non-scoring, losing a point
            if owner == 0 { self.score -= 1; }
            if owner == 1 { self.score += 1; }
        }
    }
    // inform the score counter that a swap moved a piece that might possibly affect the score
    pub fn propagate_delete(&mut self, idx: usize) {
        //assert!(self.cells[idx] > 0);
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
                    b.cells[c] = 4 + (player as u8);
                    b.hash ^= get_zobrist(c, b.cells[c]);
                    b.propagate(c);
                    yield b;
                }
            }
            for i in 0..self.cells.len() {
                // is this cell filled and upright?
                if self.cells[i] & 6 != 4 { continue; }
                for j in i+1..self.cells.len() {
                    if self.cells[j] & 6 != 4 { continue; }
                    let mut b = self.clone();
                    b.propagate_delete(i);
                    b.propagate_delete(j);
                    let (el1, el2) = (b.cells[i], b.cells[j]);
                    b.cells.swap(i,j);
                    b.cells[i] |= 2;
                    b.cells[j] |= 2;
                    b.hash ^= get_zobrist(i, el1) ^ get_zobrist(j, el1|2) ^ get_zobrist(j, el2) ^ get_zobrist(i, el2|2);
                    b.propagate(i);
                    b.propagate(j);
                    yield b;
                }
            }
        }.into_iter();
    }
    pub fn hash(&self) -> u64 {
        self.cells.iter().enumerate().fold(0u64, |a, (i, &e)| a ^ get_zobrist(i, e))
    }
}
type BoardHash = u64;

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "===")?;
        for (i,c) in self.cells.iter().enumerate() {
            if i != 0 && i % WIDTH == 0 { writeln!(f)?; }
            write!(f, "{}", b".???xoXO"[*c as usize] as char)?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct Solver {
    cache: ahash::AHashMap<BoardHash, ((i32,f32),Option<Board>)>,
}
impl Solver {
    pub fn minimax(&mut self, b: &Board, player: i8, depth: i32) -> ((i32,f32),Option<Board>) {
        //assert!(b.hash() == b.hash);
        //assert!(b.score() == b.score);
        if let Some((i,b)) = self.cache.get(&b.hash) { return (*i,b.clone()); }
        if depth >= 5 {
            return ((b.score, b.score as f32), None);
        }
        let f = b.clone().moves(player);
        let mut child_avg = 0.0;
        let mut best_so_far = (0,0.0);
        let mut best_board = None;
        let mut l = 0;
        for (_idx,i) in f.enumerate() {
            l += 1;
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
        }
        child_avg /= l as f32;
        best_so_far.1 = child_avg;
        if best_board.is_none() { best_so_far.0 = b.score; best_so_far.1 = best_so_far.0 as f32; }
        self.cache.insert(b.hash, (best_so_far, best_board.clone()));
        (best_so_far,best_board)
    }
}

fn init_zobrist() {
    use rand::RngCore;
    let mut rng = rand::rngs::StdRng::seed_from_u64(1337);
    for i in 0..WIDTH*HEIGHT*4 {
        unsafe {
            ZOBRIST[i] = rng.next_u64();
        }
    }
}

fn main() {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    let mut b = Board::new();
    let mut solver = Solver::default();
    let mut turn = 0;
    init_zobrist();
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
