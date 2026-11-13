pub const BLOCK: usize = 16;
const GOB_BYTES: usize = 512;
const GOB_ROWS: usize = 8;
const GOB_BLOCKS: usize = 4;

pub fn addr(x: usize, y: usize, wb: usize, gobs: usize) -> usize {
    let gobs: usize = gobs.max(1);
    let stride: usize = wb.div_ceil(GOB_BLOCKS);
    let rob: usize = GOB_ROWS * gobs;
    (y / rob) * GOB_BYTES * gobs * stride
        + (x / GOB_BLOCKS) * GOB_BYTES * gobs
        + ((y % rob) / GOB_ROWS) * GOB_BYTES
        + (y & 1) * 16
        + ((y >> 1) & 1) * 32
        + (x & 1) * 64
        + ((y >> 2) & 1) * 128
        + ((x >> 1) & 1) * 256
}

pub fn span(wb: usize, hb: usize, gobs: usize) -> usize {
    if wb == 0 || hb == 0 {
        return 0;
    }
    let gobs: usize = gobs.max(1);
    let stride: usize = wb.div_ceil(GOB_BLOCKS);
    hb.div_ceil(GOB_ROWS * gobs) * GOB_BYTES * gobs * stride
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_first_gob_follows_the_measured_order() {
        let want: [(usize, usize, usize); 9] = [(0, 0, 0), (0, 1, 16), (0, 2, 32), (0, 3, 48), (1, 0, 64), (0, 4, 128), (2, 0, 256), (3, 7, 496), (4, 0, 512)];
        for (x, y, a) in want {
            assert_eq!(addr(x, y, 16, 1), a, "{x} {y}");
        }
    }

    #[test]
    fn every_block_gets_its_own_slot_inside_the_span() {
        for (wb, hb, gobs) in [(1, 1, 1), (5, 9, 1), (27, 27, 4), (86, 86, 16), (33, 70, 2), (4, 64, 8)] {
            let n: usize = span(wb, hb, gobs);
            let mut seen: HashSet<usize> = HashSet::new();
            for y in 0..hb {
                for x in 0..wb {
                    let a: usize = addr(x, y, wb, gobs);
                    assert_eq!(a % BLOCK, 0);
                    assert!(a + BLOCK <= n, "{wb} {hb} {gobs} {x} {y}");
                    assert!(seen.insert(a));
                }
            }
        }
        assert_eq!(span(0, 4, 1), 0);
    }

    #[test]
    fn taller_blocks_stack_gobs_before_moving_right() {
        assert_eq!(addr(0, 8, 16, 2), 512);
        assert_eq!(addr(4, 0, 16, 2), 1024);
        assert_eq!(addr(0, 16, 16, 2), 1024 * 4);
        assert_eq!(addr(0, 8, 16, 1), 512 * 4);
    }
}
