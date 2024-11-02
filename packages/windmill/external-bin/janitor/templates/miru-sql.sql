
SET @saved_cs_client     = @@character_set_client;

SET character_set_client = @saved_cs_client;

DROP TABLE IF EXISTS `boc_members`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `boc_members` (
  `BOC_ID` varchar(128) NOT NULL,
  `BOC_NAME` varchar(128) DEFAULT NULL,
  `BOC_ROLE` varchar(2) DEFAULT NULL,
  `CCS_CODE` varchar(255) DEFAULT NULL,
  `CERT_ALIAS` varchar(255) DEFAULT NULL,
  `LAST_MOD_TS` datetime NOT NULL,
  PRIMARY KEY (`BOC_ID`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1 ROW_FORMAT=DYNAMIC;

--
-- Dumping data for table `boc_members`
--

LOCK TABLES `boc_members` WRITE;
/*!40000 ALTER TABLE `boc_members` DISABLE KEYS */;
{{{boc_members}}}
/*!40000 ALTER TABLE `boc_members` ENABLE KEYS */;
UNLOCK TABLES;


DROP TABLE IF EXISTS `candidates`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `candidates` (
  `CANDIDATE_CODE` varchar(32) NOT NULL,
  `CANDIDATE_ID` varchar(16) NOT NULL,
  `LAST_NAME` varchar(300) DEFAULT NULL,
  `FIRST_NAME` varchar(200) DEFAULT NULL,
  `MATERNAL_NAME` varchar(200) DEFAULT NULL,
  `NICKNAME` varchar(128) DEFAULT NULL,
  `NAME_ON_BALLOT` varchar(128) DEFAULT NULL,
  `GENDER` varchar(1) DEFAULT NULL,
  `CONTEST_CODE` varchar(8) DEFAULT NULL,
  `MANUAL_ORDER` int DEFAULT NULL,
  PRIMARY KEY (`CANDIDATE_ID`) USING BTREE,
  UNIQUE KEY `CAND_CODE_UNIQUE` (`CANDIDATE_CODE`),
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 COMMENT='this table contains crucial information of candidates';

--
-- Dumping data for table `candidates`
--

LOCK TABLES `candidates` WRITE;
/*!40000 ALTER TABLE `candidates` DISABLE KEYS */;
{{{candidates}}}
/*!40000 ALTER TABLE `candidates` ENABLE KEYS */;
UNLOCK TABLES;

DROP TABLE IF EXISTS `ccs`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `ccs` (
  `CCS_CODE` varchar(32) NOT NULL,
  `CCS_ID` varchar(128) NOT NULL,
  `CCS_URL` varchar(300) DEFAULT NULL,
  `BOARD_TYPE` varchar(8) DEFAULT NULL,
  `TALLY_TYPE` varchar(1) DEFAULT NULL,
  `HUC` varchar(3) DEFAULT NULL,
  `REGION_CODE` varchar(8) DEFAULT NULL,
  `UPPER_CCS` varchar(128) DEFAULT NULL,
  `LAST_MOD_TS` datetime NOT NULL,
  PRIMARY KEY (`CCS_ID`) USING BTREE,
  UNIQUE KEY `CCS_CODE_UNIQUE` (`CCS_CODE`),
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;

--
-- Dumping data for table `ccs`
--

LOCK TABLES `ccs` WRITE;
/*!40000 ALTER TABLE `ccs` DISABLE KEYS */;
{{{ccs}}}
/*!40000 ALTER TABLE `ccs` ENABLE KEYS */;
UNLOCK TABLES;

DROP TABLE IF EXISTS `contest`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `contest` (
  `CONTEST_CODE` varchar(32) NOT NULL,
  `CONTEST_NAME` varchar(200) NOT NULL,
  `CONTEST_CLASS_CODE` varchar(8) DEFAULT NULL,
  `POLLING_DISTRICT_CODE` varchar(16) DEFAULT NULL,
  `SUMMARY` varchar(100) DEFAULT NULL,
  `CCS_CODE` varchar(16) DEFAULT NULL,
  `LAST_MOD_TS` datetime NOT NULL,
  PRIMARY KEY (`CONTEST_CODE`) USING BTREE,
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;

--
-- Dumping data for table `contest`
--

LOCK TABLES `contest` WRITE;
/*!40000 ALTER TABLE `contest` DISABLE KEYS */;
{{{contest}}}
/*!40000 ALTER TABLE `contest` ENABLE KEYS */;
UNLOCK TABLES;


DROP TABLE IF EXISTS `eb_members`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `eb_members` (
  `EB_ID` varchar(128) NOT NULL,
  `EB_NAME` varchar(128) NOT NULL,
  `EB_ROLE` varchar(2) DEFAULT NULL,
  `PRECINCT_CODE` varchar(16) DEFAULT NULL,
  `CERT_ALIAS` varchar(32) DEFAULT NULL,
  `LAST_MOD_TS` datetime NOT NULL,
  PRIMARY KEY (`EB_ID`) USING BTREE,
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;

--
-- Dumping data for table `eb_members`
--

LOCK TABLES `eb_members` WRITE;
/*!40000 ALTER TABLE `eb_members` DISABLE KEYS */;
{{{eb_members}}}
/*!40000 ALTER TABLE `eb_members` ENABLE KEYS */;
UNLOCK TABLES;



DROP TABLE IF EXISTS `political_organizations`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `political_organizations` (
  `POLITICAL_ORG_CODE` varchar(32) NOT NULL,
  `POLITICAL_ORG_NAME` varchar(128) NOT NULL,
  `INITIALS` varchar(32) DEFAULT NULL,
  `DESCRIPTION` varchar(128) DEFAULT NULL,
  PRIMARY KEY (`POLITICAL_ORG_CODE`) USING BTREE,
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;

--
-- Dumping data for table `political_organizations`
--

LOCK TABLES `political_organizations` WRITE;
/*!40000 ALTER TABLE `political_organizations` DISABLE KEYS */;
{{{political_organizations}}}
/*!40000 ALTER TABLE `political_organizations` ENABLE KEYS */;
UNLOCK TABLES;


DROP TABLE IF EXISTS `polling_centers`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `polling_centers` (
  `VOTING_CENTER_CODE` varchar(32) NOT NULL,
  `VOTING_CENTER_NAME` varchar(128) NOT NULL,
  `DESCRIPTION` varchar(512) DEFAULT NULL,
  `VOTING_CENTER_ADDR` varchar(512) DEFAULT NULL,
  `REGISTERED_VOTERS` int DEFAULT NULL,
  `CITY` varchar(64) DEFAULT NULL,
  `REGION_CODE` varchar(32) DEFAULT NULL,
  `LAST_MOD_TS` datetime NOT NULL,
  PRIMARY KEY (`VOTING_CENTER_CODE`) USING BTREE,
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;

--
-- Dumping data for table `polling_centers`
--

LOCK TABLES `polling_centers` WRITE;
/*!40000 ALTER TABLE `polling_centers` DISABLE KEYS */;
{{{polling_centers}}}
/*!40000 ALTER TABLE `polling_centers` ENABLE KEYS */;
UNLOCK TABLES;

DROP TABLE IF EXISTS `polling_district_region`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `polling_district_region` (
  `POLLING_DISTRICT_CODE` varchar(32) NOT NULL,
  `REGION_CODE` varchar(32) DEFAULT NULL,
  `LAST_MOD_TS` datetime NOT NULL,
  PRIMARY KEY (`POLLING_DISTRICT_CODE`) USING BTREE,
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;

--
-- Dumping data for table `polling_district_region`
--

LOCK TABLES `polling_district_region` WRITE;
/*!40000 ALTER TABLE `polling_district_region` DISABLE KEYS */;
{{{polling_district_region}}}
/*!40000 ALTER TABLE `polling_district_region` ENABLE KEYS */;
UNLOCK TABLES;


DROP TABLE IF EXISTS `polling_district`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `polling_district` (
  `POLLING_DISTRICT_CODE` varchar(32) NOT NULL,
  `POLLING_DISTRICT_NAME` varchar(64) NOT NULL,
  `DESCRIPTION` varchar(200) DEFAULT NULL,
  `POLLING_DISTRICT_NUMBER` int DEFAULT NULL,
  `LAST_MOD_TS` datetime NOT NULL,
  PRIMARY KEY (`POLLING_DISTRICT_CODE`) USING BTREE,
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;

--
-- Dumping data for table `polling_district`
--

LOCK TABLES `polling_district` WRITE;
/*!40000 ALTER TABLE `polling_district` DISABLE KEYS */;
{{{polling_district}}}
/*!40000 ALTER TABLE `polling_district` ENABLE KEYS */;
UNLOCK TABLES;

DROP TABLE IF EXISTS `precinct_established`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `precinct_established` (
  `PRECINCT_CODE` varchar(32) NOT NULL,
  `ESTABLISHED_CODE` varchar(32) NOT NULL,
  `DESCRIPTION` varchar(512) DEFAULT NULL,
  `REGISTERED_VOTERS` int DEFAULT NULL,
  `REGION_CODE` varchar(32) DEFAULT NULL,
  `LAST_MOD_TS` datetime NOT NULL,
  PRIMARY KEY (`PRECINCT_CODE`, `ESTABLISHED_CODE`),
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;

--
-- Dumping data for table `precinct_established`
--

LOCK TABLES `precinct_established` WRITE;
/*!40000 ALTER TABLE `precinct_established` DISABLE KEYS */;
{{{precinct_established}}}
/*!40000 ALTER TABLE `precinct_established` ENABLE KEYS */;
UNLOCK TABLES;
