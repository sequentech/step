
pub struct BallotCodec {
    bases: Vec<u32>,
}

impl BallotCodec {
    pub fn new(bases: Vec<u32>) -> Self {
        Self { bases }
    }

    pub fn encode_ballot(&self, choices: Vec<u32>) -> u32 {
        if choices.len() != self.bases.len() {
            panic!("choices doesn't match with bases");
        }

        let encoded_choices = choices
            .iter()
            .enumerate()
            .map(|(i, &choice)| {
                let base_mul = self.bases.iter().take(i + 1).product::<u32>();
                choice * base_mul
            })
            .sum();

        encoded_choices
    }

    pub fn decode_ballot(&self, encoded: u32) -> Vec<u32> {
        let mut choices: Vec<u32> = vec![];
        let mut remaining = encoded;

        let enum_bases: Vec<u32> = self
            .bases
            .iter()
            .enumerate()
            .map(|(i, &_base)| self.bases.iter().take(i + 1).product::<u32>())
            .collect::<Vec<u32>>();

        for &base in enum_bases.iter().rev() {
            choices.push(remaining / base);
            remaining %= base;
        }
        choices.reverse();

        choices
    }
}
