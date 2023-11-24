type Option  = {
    id: string
    name: string
}
type VotingTypes = Option[]
type CountingAlgorithms = Option[]
type OrderAnwers = Option[]

export const VOTING_TYPES = (t: any) => [{id: "no-preferential", name: t("contestScreen.options.no-preferential")}]

export const COUNTING_ALGORITHMS = (t: any) => [
    {id: "plurality-at-large", name: t("contestScreen.options.plurality-at-large")},
]

export const ORDER_ANSWERS = (t: any) => [
    {id: "random-asnwers", name: t("contestScreen.options.random-asnwers")},
    {id: "custom", name: t("contestScreen.options.custom")},
    {id: "alphabetical", name: t("contestScreen.options.alphabetical")},
]
