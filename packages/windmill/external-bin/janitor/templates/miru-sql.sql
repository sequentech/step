
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

