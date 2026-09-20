pub(super) struct Occurrence {
    pub site: usize,
    pub count: usize,
}

pub(super) fn intersect<'index>(
    posting: &'index [&'index [Occurrence]],
    rank: &'index [usize],
) -> impl Iterator<Item = usize> + 'index {
    let (anchor, candidate) = posting
        .iter()
        .copied()
        .enumerate()
        .min_by_key(|(_, posting)| posting.len())
        .unwrap();
    let mut cursor: smallvec::SmallVec<[usize; 4]> = smallvec::smallvec![0; posting.len()];
    candidate
        .iter()
        .filter(move |occurrence| {
            let site = occurrence.site;
            posting.iter().enumerate().all(|(position, posting)| {
                if position == anchor {
                    return true;
                }
                if candidate.len() < 16 {
                    return if posting.len() <= 16 {
                        posting.iter().any(|occurrence| occurrence.site == site)
                    } else {
                        posting
                            .binary_search_by_key(&rank[site], |occurrence| rank[occurrence.site])
                            .is_ok()
                    };
                }
                let cursor: &mut usize = &mut cursor[position];
                if posting.len() / candidate.len() <= 8 {
                    while posting
                        .get(*cursor)
                        .is_some_and(|occurrence| rank[occurrence.site] < rank[site])
                    {
                        *cursor += 1;
                    }
                } else {
                    *cursor += posting[*cursor..]
                        .partition_point(|occurrence| rank[occurrence.site] < rank[site]);
                }
                posting
                    .get(*cursor)
                    .is_some_and(|occurrence| occurrence.site == site)
            })
        })
        .map(|occurrence| occurrence.site)
}
