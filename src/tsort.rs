use std::collections::BTreeMap;

struct Pair(usize, usize);
/// topological sorting
/// 
pub struct TopSort {
    pairs: Vec<Pair>,
}

#[derive(Debug)]
struct Node {
    count: usize,
    // k: usize,
    succ: Vec<usize>,
}
impl TopSort {
    pub fn new() -> Self {
        Self { pairs: vec![] }
    }

    pub fn add(&mut self, pred: usize, succ: usize) {
        self.pairs.push(Pair(pred, succ));
    }

    pub fn sorted(&self) -> Vec<usize> {
        let mut result = vec![];
        let mut map = BTreeMap::<usize, Node>::new();
        for x in self.pairs.iter() {
            if !map.contains_key(&x.0) {
                map.insert(
                    x.0,
                    Node {
                        count: 0,
                        // k: x.0,
                        succ: vec![],
                    },
                );
            }
            let p = map.get_mut(&x.0).unwrap();
            p.succ.push(x.1);

            if !map.contains_key(&x.1) {
                map.insert(
                    x.1,
                    Node {
                        count: 0,
                        // k: x.1,
                        succ: vec![],
                    },
                );
            }
            let p = map.get_mut(&x.1).unwrap();
            p.count += 1;
        }

        while map.len() > 0 {
            let zeros: Vec<usize> = map
                .iter()
                .filter(|(_, x)| x.count == 0)
                .map(|(x, _)| *x)
                .collect();

            for x in zeros {
                result.push(x);
                let succ = {
                    let e = map.get(&x).unwrap();
                    let succ = e.succ.clone();
                    map.remove(&x);
                    succ
                };

                for y in succ.iter() {
                    let f = map.get_mut(y).unwrap();
                    f.count -= 1;
                }
            }
        }
        result
    }
}

#[test]
fn test1() {
    let mut tsort = TopSort::new();
    tsort.add(9, 2);
    tsort.add(3, 7);
    tsort.add(7, 5);
    tsort.add(5, 8);
    tsort.add(8, 6);
    tsort.add(4, 6);
    tsort.add(1, 3);
    tsort.add(7, 4);
    tsort.add(9, 5);
    tsort.add(2, 8);
    assert_eq!(tsort.sorted(), vec![1, 9, 2, 3, 7, 4, 5, 8, 6]);
}
