type VotingType  = typeof VOTING_TYPES[0]
type VotingTypes = VotingType[]

export const VOTING_TYPES = [
    {id: "no-preferential", name: "no-preferential"},
]

type CountingAlgorithm  = typeof COUNTING_ALGORITHMS[0]
type CountingAlgorithms = CountingAlgorithm[]

export const COUNTING_ALGORITHMS = [
    {id: "plurality-at-large", name: "plurality-at-large"},
]

type OrderAnwer  = typeof ORDER_ANSWERS[0]
type OrderAnwers = OrderAnwer[]

export const ORDER_ANSWERS = [
    {id: "random-asnwers", name: "random-asnwers"},
    {id: "custom", name: "custom"},
    {id: "alphabetical", name: "alphabetical"},
]
