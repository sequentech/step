--
-- PostgreSQL database dump
--
-- pg_dump --host postgres --user=postgres --password --data-only --schema=sequent_backend > sample_data.sql
-- psql --host postgres --user=postgres --password < src/sql/sample_data.sql

-- Dumped from database version 15.3 (Debian 15.3-1.pgdg110+1)
-- Dumped by pg_dump version 15.3

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Data for Name: tenant; Type: TABLE DATA; Schema: sequent_backend; Owner: postgres
--

COPY sequent_backend.tenant (id, username, created_at, updated_at, labels, annotations, is_active) FROM stdin;
11734286-8c16-47a4-9777-b985de3daf6c	username2	2023-08-06 19:14:31.122786+00	2023-08-06 19:14:31.122786+00	\N	\N	t
4882096d-65f7-478f-b334-a0bfbda73d22	username1	2023-08-06 19:01:58.2281+00	2023-08-06 23:45:44.348349+00	\N	\N	t
\.


--
-- Data for Name: election_event; Type: TABLE DATA; Schema: sequent_backend; Owner: postgres
--

COPY sequent_backend.election_event (id, created_at, updated_at, labels, annotations, tenant_id, name, description, presentation, bulletin_board_reference, is_archived, voting_channels, dates, status, user_boards, encryption_protocol, is_audit, audit_election_event_id) FROM stdin;
efc8b53a-076d-4270-91cb-ed3a807058e9	2023-08-06 19:12:50.016647+00	2023-08-06 23:44:12.890397+00	\N	\N	4882096d-65f7-478f-b334-a0bfbda73d22	event1	Event 1	\N	\N	f	\N	\N	\N	user_boards	mixnet	f	\N
f4cf58c9-b991-40b7-9a4e-0ce6eff8cf9b	2023-08-06 19:13:18.443437+00	2023-08-06 23:44:26.634354+00	\N	\N	4882096d-65f7-478f-b334-a0bfbda73d22	event2	Event 2	\N	\N	f	\N	\N	\N	user_boards	mixnet	f	\N
5784f145-faec-4569-b4cf-928265bfad48	2023-08-06 19:17:00.605099+00	2023-08-06 23:44:59.079114+00	\N	\N	11734286-8c16-47a4-9777-b985de3daf6c	event3	Event 3	\N	\N	f	\N	\N	\N	user_boards	mixnet	f	\N
d63ed5fa-d6bb-4a84-885a-a7dca594de9b	2023-08-06 19:17:08.336123+00	2023-08-06 23:45:14.139174+00	\N	\N	11734286-8c16-47a4-9777-b985de3daf6c	event4	Event 4	\N	\N	f	\N	\N	\N	user_boards	mixnet	f	\N
\.


--
-- Data for Name: area; Type: TABLE DATA; Schema: sequent_backend; Owner: postgres
--

COPY sequent_backend.area (id, tenant_id, election_event_id, created_at, last_updated_at, labels, annotations, name, description, type) FROM stdin;
76d14e0b-456f-48e0-9b1b-d589d811bc5f	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	2023-08-06 22:26:35.051401+00	2023-08-06 22:26:35.051401+00	\N	\N	event1.area1	Area 1 for event 1	areatype
70b810d2-8302-4b82-b219-7e3e1e5efd80	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	2023-08-06 22:26:59.531679+00	2023-08-06 22:26:59.531679+00	\N	\N	event1.area2	Area 2 for event 1	areatype
5529ec06-a01a-408c-bc68-aaf6249fa62e	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	2023-08-06 22:27:06.585168+00	2023-08-06 22:27:06.585168+00	\N	\N	event1.area3	Area 3 for event 1	areatype
134027a8-d3e1-478f-bb31-1cf9f69fe712	4882096d-65f7-478f-b334-a0bfbda73d22	f4cf58c9-b991-40b7-9a4e-0ce6eff8cf9b	2023-08-06 22:27:35.628815+00	2023-08-06 22:27:35.628815+00	\N	\N	event2.area1	Area 1 for event 2	areatype
a720b652-0d5e-4d64-a366-9aacb3edcdb1	4882096d-65f7-478f-b334-a0bfbda73d22	f4cf58c9-b991-40b7-9a4e-0ce6eff8cf9b	2023-08-06 22:27:42.962359+00	2023-08-06 22:27:42.962359+00	\N	\N	event2.area2	Area 2 for event 2	areatype
144a5fa1-a641-45fa-98d8-729078985a9c	4882096d-65f7-478f-b334-a0bfbda73d22	f4cf58c9-b991-40b7-9a4e-0ce6eff8cf9b	2023-08-06 22:27:50.530573+00	2023-08-06 22:27:50.530573+00	\N	\N	event2.area3	Area 3 for event 2	areatype
09ee303c-1d45-46f2-9f81-19265ca8ecb7	11734286-8c16-47a4-9777-b985de3daf6c	5784f145-faec-4569-b4cf-928265bfad48	2023-08-06 22:28:16.454856+00	2023-08-06 22:28:16.454856+00	\N	\N	event3.area1	Area 1 for event 3	areatype
7134f45b-5dc1-4367-bec5-af99352d1242	11734286-8c16-47a4-9777-b985de3daf6c	5784f145-faec-4569-b4cf-928265bfad48	2023-08-06 22:28:23.181269+00	2023-08-06 22:28:23.181269+00	\N	\N	event3.area2	Area 2 for event 3	areatype
a085bca4-52b3-4ad5-8aed-84c5641543e1	11734286-8c16-47a4-9777-b985de3daf6c	5784f145-faec-4569-b4cf-928265bfad48	2023-08-06 22:28:30.529116+00	2023-08-06 22:28:30.529116+00	\N	\N	event3.area3	Area 3 for event 3	areatype
e4f63d5b-ea96-40b7-8d2e-2dc8c9cfc283	11734286-8c16-47a4-9777-b985de3daf6c	d63ed5fa-d6bb-4a84-885a-a7dca594de9b	2023-08-06 22:28:46.668133+00	2023-08-06 22:28:46.668133+00	\N	\N	event4.area1	Area 1 for event 4	areatype
ed72d58a-1efd-499d-8387-89066d049698	11734286-8c16-47a4-9777-b985de3daf6c	d63ed5fa-d6bb-4a84-885a-a7dca594de9b	2023-08-06 22:28:52.47554+00	2023-08-06 22:28:52.47554+00	\N	\N	event4.area2	Area 2 for event 4	areatype
f89eb1c1-a8d0-42ff-9eb9-d59a645d2db0	11734286-8c16-47a4-9777-b985de3daf6c	d63ed5fa-d6bb-4a84-885a-a7dca594de9b	2023-08-06 22:29:00.679899+00	2023-08-06 22:29:00.679899+00	\N	\N	event4.area3	Area 3 for event 4	areatype
\.


--
-- Data for Name: election; Type: TABLE DATA; Schema: sequent_backend; Owner: postgres
--

COPY sequent_backend.election (id, tenant_id, election_event_id, created_at, last_updated_at, labels, annotations, name, description, presentation, dates, status, eml, num_allowed_revotes, is_consolidated_ballot_encoding, spoil_ballot_option) FROM stdin;
96814f22-47dd-428b-9e34-6f8b882be59f	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	2023-08-06 22:20:52.030167+00	2023-08-06 22:20:52.030167+00	\N	\N	event1.election1	Election 1 for event 1	\N	\N	\N	ELECTION_EML: Election 1 for event 1	1	f	f
ead21f69-6d66-4d2c-9d82-8f55cbcce613	4882096d-65f7-478f-b334-a0bfbda73d22	f4cf58c9-b991-40b7-9a4e-0ce6eff8cf9b	2023-08-06 22:22:20.853742+00	2023-08-06 22:22:20.853742+00	\N	\N	event2.election1	Election 1 for event 2	\N	\N	\N	ELECTION_EML: Election 1 for event 2	1	f	f
fbea27f2-13c6-4600-83ef-ec9ee6e58517	4882096d-65f7-478f-b334-a0bfbda73d22	f4cf58c9-b991-40b7-9a4e-0ce6eff8cf9b	2023-08-06 22:22:37.275641+00	2023-08-06 22:22:37.275641+00	\N	\N	event2.election2	Election 2 for event 2	\N	\N	\N	ELECTION_EML: Election 2 for event 2	1	f	f
ef8ea02f-eb87-48ba-8339-eb35f4e46781	11734286-8c16-47a4-9777-b985de3daf6c	5784f145-faec-4569-b4cf-928265bfad48	2023-08-06 22:23:36.313967+00	2023-08-06 22:23:36.313967+00	\N	\N	event3.election1	Election 1 for event 3	\N	\N	\N	ELECTION_EML: Election 1 for event 3	1	f	f
c6379782-0614-41ef-b931-31a8df6cffad	11734286-8c16-47a4-9777-b985de3daf6c	5784f145-faec-4569-b4cf-928265bfad48	2023-08-06 22:23:44.722888+00	2023-08-06 22:23:44.722888+00	\N	\N	event3.election2	Election 2 for event 3	\N	\N	\N	ELECTION_EML: Election 2 for event 3	1	f	f
d7da5b6b-344c-4ad4-a5c8-bf14d996111f	11734286-8c16-47a4-9777-b985de3daf6c	d63ed5fa-d6bb-4a84-885a-a7dca594de9b	2023-08-06 22:24:09.457514+00	2023-08-06 22:24:09.457514+00	\N	\N	event4.election1	Election 1 for event 4	\N	\N	\N	ELECTION_EML: Election 1 for event 4	1	f	f
dc792bad-bc6b-4665-926d-beefeb2259ff	11734286-8c16-47a4-9777-b985de3daf6c	d63ed5fa-d6bb-4a84-885a-a7dca594de9b	2023-08-06 22:24:15.167863+00	2023-08-06 22:24:15.167863+00	\N	\N	event4.election2	Election 2 for event 4	\N	\N	\N	ELECTION_EML: Election 2 for event 4	1	f	f
007e88dc-9aa0-44df-81d3-57568f63ba2c	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	2023-08-06 22:21:22.307332+00	2023-08-06 22:21:22.307332+00	\N	\N	event1.election2	Election 2 for event 1	\N	\N	\N	ELECTION_EML: Election 2 for event 1	1	f	f
\.


--
-- Data for Name: contest; Type: TABLE DATA; Schema: sequent_backend; Owner: postgres
--

COPY sequent_backend.contest (id, tenant_id, election_event_id, election_id, created_at, last_updated_at, labels, annotations, is_acclaimed, is_active, name, description, presentation, min_votes, max_votes, voting_type, counting_algorithm, is_encrypted, tally_configuration, conditions) FROM stdin;
e2cefc41-d718-4655-b35a-e55aaa475821	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	96814f22-47dd-428b-9e34-6f8b882be59f	2023-08-06 22:40:37.472786+00	2023-08-06 22:40:37.472786+00	\N	\N	f	t	event1.election1.contest1	Contest 1 for election 1 for event 1	\N	1	4	plurality	plurality	t	\N	\N
fb1c067c-4487-4a8a-8457-0eeeb87da6fe	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	96814f22-47dd-428b-9e34-6f8b882be59f	2023-08-06 22:41:18.028157+00	2023-08-06 22:41:18.028157+00	\N	\N	f	t	event1.election1.contest2	Contest 2 for election 1 for event 1	\N	1	4	plurality	plurality	t	\N	\N
cb44438c-137a-4a3b-92a7-c212865682ff	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	96814f22-47dd-428b-9e34-6f8b882be59f	2023-08-06 22:41:31.608473+00	2023-08-06 22:41:31.608473+00	\N	\N	f	t	event1.election1.contest3	Contest 3 for election 1 for event 1	\N	1	4	plurality	plurality	t	\N	\N
\.


--
-- Data for Name: area_contest; Type: TABLE DATA; Schema: sequent_backend; Owner: postgres
--

COPY sequent_backend.area_contest (id, tenant_id, election_event_id, contest_id, area_id, created_at, last_updated_at, labels, annotations) FROM stdin;
05f5c0b6-d62a-46f9-a6f0-c5c5a4fa81e4	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	e2cefc41-d718-4655-b35a-e55aaa475821	76d14e0b-456f-48e0-9b1b-d589d811bc5f	2023-08-06 23:00:41.756212+00	2023-08-06 23:00:41.756212+00	\N	\N
36fb99ea-9f93-4980-8da2-96ebe995b6fd	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	e2cefc41-d718-4655-b35a-e55aaa475821	70b810d2-8302-4b82-b219-7e3e1e5efd80	2023-08-06 23:01:35.231815+00	2023-08-06 23:01:35.231815+00	\N	\N
b1cd27bd-826e-47b6-bfeb-df04834429ed	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	e2cefc41-d718-4655-b35a-e55aaa475821	5529ec06-a01a-408c-bc68-aaf6249fa62e	2023-08-06 23:01:51.206517+00	2023-08-06 23:01:51.206517+00	\N	\N
7f7cb08b-c63b-42fc-be8f-9c297ab36907	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	fb1c067c-4487-4a8a-8457-0eeeb87da6fe	70b810d2-8302-4b82-b219-7e3e1e5efd80	2023-08-06 23:02:20.298799+00	2023-08-06 23:02:20.298799+00	\N	\N
c9ce4491-3a33-47e2-bb7b-513439043a52	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	cb44438c-137a-4a3b-92a7-c212865682ff	5529ec06-a01a-408c-bc68-aaf6249fa62e	2023-08-06 23:02:44.892254+00	2023-08-06 23:02:44.892254+00	\N	\N
\.


--
-- Data for Name: ballot_style; Type: TABLE DATA; Schema: sequent_backend; Owner: postgres
--

COPY sequent_backend.ballot_style (id, tenant_id, election_id, area_id, created_at, last_updated_at, labels, annotations, ballot_eml, ballot_signature, status, election_event_id) FROM stdin;
bfdbf93a-f5ab-480e-b524-d49755bdc361	4882096d-65f7-478f-b334-a0bfbda73d22	96814f22-47dd-428b-9e34-6f8b882be59f	76d14e0b-456f-48e0-9b1b-d589d811bc5f	2023-08-06 23:22:50.088546+00	2023-08-06 23:22:50.088546+00	\N	\N	BALLOT_STYLE_EML: Contest 1	\N	ballotstylestatus	efc8b53a-076d-4270-91cb-ed3a807058e9
9f66682a-c970-4a27-94a8-7cc202d2d8eb	4882096d-65f7-478f-b334-a0bfbda73d22	96814f22-47dd-428b-9e34-6f8b882be59f	70b810d2-8302-4b82-b219-7e3e1e5efd80	2023-08-06 23:23:05.037571+00	2023-08-06 23:23:05.037571+00	\N	\N	BALLOT_STYLE_EML: Contest 1,2	\N	ballotstylestatus	efc8b53a-076d-4270-91cb-ed3a807058e9
04eb7af9-4a7f-4b09-9bf1-504ce8f4f875	4882096d-65f7-478f-b334-a0bfbda73d22	96814f22-47dd-428b-9e34-6f8b882be59f	5529ec06-a01a-408c-bc68-aaf6249fa62e	2023-08-06 23:23:24.239179+00	2023-08-06 23:23:24.239179+00	\N	\N	BALLOT_STYLE_EML: Contest 1,3	\N	ballotstylestatus	efc8b53a-076d-4270-91cb-ed3a807058e9
\.


--
-- Data for Name: candidate; Type: TABLE DATA; Schema: sequent_backend; Owner: postgres
--

COPY sequent_backend.candidate (id, tenant_id, election_event_id, contest_id, created_at, last_updated_at, labels, annotations, name, description, type, presentation, is_public) FROM stdin;
8f714e7b-cce9-4158-880e-49a7c7d92eeb	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	e2cefc41-d718-4655-b35a-e55aaa475821	2023-08-06 23:07:45.908207+00	2023-08-06 23:07:45.908207+00	\N	\N	event1.election1.contest1.candidate1	Candidate 1 for contest 1	candidatetype	\N	f
0e099968-eede-43fd-92d3-b09d8eae6b95	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	e2cefc41-d718-4655-b35a-e55aaa475821	2023-08-06 23:07:54.026636+00	2023-08-06 23:07:54.026636+00	\N	\N	event1.election1.contest1.candidate2	Candidate 2 for contest 1	candidatetype	\N	f
b547a18b-5e46-40c7-9360-1188928fda6f	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	fb1c067c-4487-4a8a-8457-0eeeb87da6fe	2023-08-06 23:08:33.571406+00	2023-08-06 23:08:33.571406+00	\N	\N	event1.election1.contest2.candidate1	Candidate 1 for contest 2	candidatetype	\N	t
b5890a4c-683f-4d78-8a77-d6bfb6657108	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	fb1c067c-4487-4a8a-8457-0eeeb87da6fe	2023-08-06 23:08:42.036078+00	2023-08-06 23:08:42.036078+00	\N	\N	event1.election1.contest2.candidate2	Candidate 2 for contest 2	candidatetype	\N	t
fe095b9d-13a5-483e-b91d-6856c1ca07bc	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	cb44438c-137a-4a3b-92a7-c212865682ff	2023-08-06 23:09:01.955157+00	2023-08-06 23:09:01.955157+00	\N	\N	event1.election1.contest3.candidate1	Candidate 1 for contest 3	candidatetype	\N	t
ffcca451-5e6a-41dd-a7dc-c3714c222a29	4882096d-65f7-478f-b334-a0bfbda73d22	efc8b53a-076d-4270-91cb-ed3a807058e9	cb44438c-137a-4a3b-92a7-c212865682ff	2023-08-06 23:09:12.004174+00	2023-08-06 23:09:12.004174+00	\N	\N	event1.election1.contest3.candidate2	Candidate 2 for contest 3	candidatetype	\N	t
\.


--
-- Data for Name: cast_vote; Type: TABLE DATA; Schema: sequent_backend; Owner: postgres
--

COPY sequent_backend.cast_vote (id, tenant_id, election_id, area_id, created_at, last_updated_at, labels, annotations, content, cast_ballot_signature, voter_id_string, election_event_id) FROM stdin;
9e54f389-64e8-4f4a-886f-6cc22972a445	4882096d-65f7-478f-b334-a0bfbda73d22	96814f22-47dd-428b-9e34-6f8b882be59f	76d14e0b-456f-48e0-9b1b-d589d811bc5f	2023-08-06 23:31:48.2242+00	2023-08-06 23:31:48.2242+00	\N	\N	VOTE_EML: Contest 1 for area1.voter1	\N	area1.voter1	efc8b53a-076d-4270-91cb-ed3a807058e9
9673c6f8-25cf-4468-8a5b-1ff8a1c98399	4882096d-65f7-478f-b334-a0bfbda73d22	96814f22-47dd-428b-9e34-6f8b882be59f	76d14e0b-456f-48e0-9b1b-d589d811bc5f	2023-08-06 23:32:14.105072+00	2023-08-06 23:32:14.105072+00	\N	\N	VOTE_EML: Contest 1 for area1.voter2	\N	area1.voter2	efc8b53a-076d-4270-91cb-ed3a807058e9
32810532-b1f2-4c61-8515-1dc19950ee0e	4882096d-65f7-478f-b334-a0bfbda73d22	96814f22-47dd-428b-9e34-6f8b882be59f	70b810d2-8302-4b82-b219-7e3e1e5efd80	2023-08-06 23:32:32.656272+00	2023-08-06 23:32:32.656272+00	\N	\N	VOTE_EML: Contest 1,2 for area2.voter1	\N	area2.voter1	efc8b53a-076d-4270-91cb-ed3a807058e9
dcb739f9-e22c-49b0-8d89-f39322a35e9c	4882096d-65f7-478f-b334-a0bfbda73d22	96814f22-47dd-428b-9e34-6f8b882be59f	70b810d2-8302-4b82-b219-7e3e1e5efd80	2023-08-06 23:32:43.199108+00	2023-08-06 23:32:43.199108+00	\N	\N	VOTE_EML: Contest 1,2 for area2.voter2	\N	area2.voter2	efc8b53a-076d-4270-91cb-ed3a807058e9
20abacaf-77ac-4da9-8382-c37ecbba009c	4882096d-65f7-478f-b334-a0bfbda73d22	96814f22-47dd-428b-9e34-6f8b882be59f	5529ec06-a01a-408c-bc68-aaf6249fa62e	2023-08-06 23:33:08.841027+00	2023-08-06 23:33:08.841027+00	\N	\N	VOTE_EML: Contest 1,3 for area3.voter1	\N	area3.voter1	efc8b53a-076d-4270-91cb-ed3a807058e9
d69729d9-aaee-49e5-94e8-f6cc9e0408d6	4882096d-65f7-478f-b334-a0bfbda73d22	96814f22-47dd-428b-9e34-6f8b882be59f	5529ec06-a01a-408c-bc68-aaf6249fa62e	2023-08-06 23:33:13.2739+00	2023-08-06 23:33:13.2739+00	\N	\N	VOTE_EML: Contest 1,3 for area3.voter2	\N	area3.voter2	efc8b53a-076d-4270-91cb-ed3a807058e9
\.


--
-- Data for Name: election_result; Type: TABLE DATA; Schema: sequent_backend; Owner: postgres
--

COPY sequent_backend.election_result (id, tenant_id, area_id, election_id, created_at, last_updated_at, labels, annotations, result_eml, result_eml_signature, statistics, election_event_id) FROM stdin;
86bd1a33-bda3-4be5-be16-53387d6c0204	4882096d-65f7-478f-b334-a0bfbda73d22	76d14e0b-456f-48e0-9b1b-d589d811bc5f	96814f22-47dd-428b-9e34-6f8b882be59f	2023-08-06 23:26:44.665913+00	2023-08-06 23:26:44.665913+00	\N	\N	resulteml: Contest1	\N	\N	efc8b53a-076d-4270-91cb-ed3a807058e9
42b68942-827e-4669-84ec-cf3f9d4cb922	4882096d-65f7-478f-b334-a0bfbda73d22	70b810d2-8302-4b82-b219-7e3e1e5efd80	96814f22-47dd-428b-9e34-6f8b882be59f	2023-08-06 23:27:06.293926+00	2023-08-06 23:27:06.293926+00	\N	\N	resulteml: Contest1,2	\N	\N	efc8b53a-076d-4270-91cb-ed3a807058e9
68ffab8b-9d22-45e8-8ce9-ee0ddb4b3564	4882096d-65f7-478f-b334-a0bfbda73d22	5529ec06-a01a-408c-bc68-aaf6249fa62e	96814f22-47dd-428b-9e34-6f8b882be59f	2023-08-06 23:27:22.625942+00	2023-08-06 23:27:22.625942+00	\N	\N	resulteml: Contest1,3	\N	\N	efc8b53a-076d-4270-91cb-ed3a807058e9
\.


--
-- PostgreSQL database dump complete
--

