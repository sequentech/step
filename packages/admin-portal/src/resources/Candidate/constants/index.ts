type CandidateType  = typeof CANDIDATE_TYPES[0]
type CandidateTypes = CandidateType[]

export const CANDIDATE_TYPES = [
    {id: "Candidate", name: "Candidate"},
    {id: "Option", name: "Option"},
    {id: "Write In", name: "Write In"},
    {id: "Open List", name: "Open List"},
    {id: "Closed List", name: "Closed List"},
    {id: "Semi Open List", name: "Semi Open List"},
    {id: "Invalid Vote", name: "Invalid Vote"},
    {id: "Blank Vote", name: "Blank Vote"},
]
