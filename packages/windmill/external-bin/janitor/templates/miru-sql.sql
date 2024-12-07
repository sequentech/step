
SET @saved_cs_client     = @@character_set_client;

SET character_set_client = @saved_cs_client;

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
  `POLITICAL_ORG_CODE` varchar(16) DEFAULT NULL,
  `LAST_MOD_TS` datetime NOT NULL,
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


DROP TABLE IF EXISTS `contest_class`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `contest_class` (
  `CONTEST_CLASS_CODE` varchar(32) NOT NULL,
  `CONTEST_CLASS_NAME` varchar(200) NOT NULL,
  `DESCRIPTION` varchar(200) DEFAULT NULL,
  `ELECTIVE_POSITION` varchar(4) DEFAULT NULL,
  `NUM_SEATS` int DEFAULT NULL,
  `PRECEDENCE` int DEFAULT NULL,
  `LAST_MOD_TS` datetime NOT NULL,
  PRIMARY KEY (`CONTEST_CLASS_CODE`) USING BTREE,
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;

--
-- Dumping data for table `contest_class`
--

LOCK TABLES `contest_class` WRITE;
/*!40000 ALTER TABLE `contest_class` DISABLE KEYS */;
{{{contest_class}}}
/*!40000 ALTER TABLE `contest_class` ENABLE KEYS */;
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
  PRIMARY KEY (`POLLING_DISTRICT_CODE`, `REGION_CODE`) USING BTREE,
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

DROP TABLE IF EXISTS `precinct`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `precinct` (
  `PRECINCT_CODE` varchar(32) NOT NULL,
  `OTHER_PRECINCT_CODE` varchar(32) NOT NULL,
  `ESTABLISHED_CODE` varchar(512) DEFAULT NULL,
  `ADDRESS` varchar(512) DEFAULT NULL,
  `REGION_ID` int DEFAULT NULL,
  `CITY` varchar(512) DEFAULT NULL,
  `REGION_CODE` varchar(32) DEFAULT NULL,
  `LAST_MOD_TS` datetime NOT NULL,
  PRIMARY KEY (`PRECINCT_CODE`, `ESTABLISHED_CODE`),
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;

--
-- Dumping data for table `precinct`
--

LOCK TABLES `precinct` WRITE;
/*!40000 ALTER TABLE `precinct` DISABLE KEYS */;
{{{precinct}}}
/*!40000 ALTER TABLE `precinct` ENABLE KEYS */;
UNLOCK TABLES;

DROP TABLE IF EXISTS `region`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `region` (
  `REGION_CODE` varchar(32) NOT NULL,
  `REGION_NAME` varchar(64) NOT NULL,
  `CATEGORY_CODE` varchar(8) DEFAULT NULL,
  `MASTER_REGION` varchar(32) DEFAULT NULL,
  `LAST_MOD_TS` datetime NOT NULL,
  PRIMARY KEY (`REGION_CODE`),
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;

--
-- Dumping data for table `region`
--

LOCK TABLES `region` WRITE;
/*!40000 ALTER TABLE `region` DISABLE KEYS */;
{{{region}}}
/*!40000 ALTER TABLE `region` ENABLE KEYS */;
UNLOCK TABLES;
