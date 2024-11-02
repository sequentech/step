
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
-- Dumping data for table `allbgy`
--

LOCK TABLES `boc_members` WRITE;
/*!40000 ALTER TABLE `allbgy` DISABLE KEYS */;
{{{boc_members}}}
/*!40000 ALTER TABLE `allbgy` ENABLE KEYS */;
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
-- Dumping data for table `allbgy`
--

LOCK TABLES `candidates` WRITE;
/*!40000 ALTER TABLE `allbgy` DISABLE KEYS */;
{{{candidates}}}
/*!40000 ALTER TABLE `allbgy` ENABLE KEYS */;
UNLOCK TABLES;
