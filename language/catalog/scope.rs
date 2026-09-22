pub(super) struct Scope {
    group: Vec<Group>,
    rule: Vec<usize>,
}

struct Group {
    input: usize,
    rule: std::ops::Range<usize>,
}

impl Scope {
    pub fn new(rule: &[usize], input: &[usize]) -> Self {
        let mut rule = rule.to_vec();
        rule.sort_by_key(|&rule| input[rule]);
        let mut group = Vec::new();
        let mut start = 0;
        for value in rule.chunk_by(|left, right| input[*left] == input[*right]) {
            let end = start + value.len();
            group.push(Group {
                input: input[value[0]],
                rule: start..end,
            });
            start = end;
        }
        Self { group, rule }
    }

    pub fn len(&self) -> usize {
        self.group.len()
    }

    pub fn get(&self, position: usize) -> (usize, &[usize]) {
        let group = &self.group[position];
        (group.input, &self.rule[group.rule.clone()])
    }

    pub fn position(&self, input: usize) -> Option<usize> {
        self.group
            .binary_search_by_key(&input, |group| group.input)
            .ok()
    }

    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.group.iter().map(|group| group.input)
    }

    pub fn retained(&self) -> usize {
        self.group.len() + self.rule.len()
    }
}
