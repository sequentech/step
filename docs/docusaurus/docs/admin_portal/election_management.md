---
id: election_management
title: Election Management
---

# Election Management

Welcome to the Election Management section of the Online Voting System documentation. This section is designed to guide election managers through the process of creating, configuring, and supervising elections using the Admin Portal or other tools provided by the system.

An online election system simplifies the administration of elections by digitizing the entire workflow — from setting up contests and candidates to managing voters and publishing results. Whether you're running a single election or managing multiple elections across different regions, the system offers the flexibility and control needed to ensure accuracy, transparency, and efficiency.

### Key Concepts
Understanding the core building blocks of the system is essential for successful election administration. Below are the primary components you'll encounter during the election setup and management process:

#### Election Event
An Election Event encompasses the full scope of an electoral process — from data configuration to the announcement of final results. It acts as a container for one or more elections and includes everything needed to run a complete voting process. A single system instance can manage multiple Election Events at once, whether they occur concurrently or sequentially. Each Election Event can be managed independently, enabling secure and scalable election operations.

#### Election
An Election is a specific voting activity conducted within an Election Event. It includes the actual act of voting, where registered voters select their preferred candidates or respond to ballot questions. Elections inherit their structure from the parent Election Event and function according to the parameters defined within it.

#### Contest
Contests are the fundamental decision-making units of an Election. Each Contest corresponds to a race, referendum, or ballot question that voters must decide on. Contests are assigned to specific Areas (such as districts or regions) and are associated with one or more Candidates or options. They help define what positions are up for election and who is eligible to vote in each.

#### Candidate
Candidates are individuals, parties, or options competing in a Contest. Each Candidate is linked to one or more Contests and associated with a specific geographic Area. During the election, voters express their preferences by selecting from the available Candidates, and the system tallies and reports results accordingly.
