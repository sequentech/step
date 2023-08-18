--
-- psql --host postgres --user=postgres --password < src/sql/clear.sql
--

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

DELETE FROM sequent_backend.area_contest;
DELETE FROM sequent_backend.contest;
DELETE FROM sequent_backend.ballot_style;
DELETE FROM sequent_backend.candidate;
DELETE FROM sequent_backend.cast_vote;
DELETE FROM sequent_backend.election_result;
DELETE FROM sequent_backend.area;
DELETE FROM sequent_backend.election;
DELETE FROM sequent_backend.election_event;
DELETE FROM sequent_backend.tenant;