import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import HomepageFeatures from '../components/HomepageFeatures';
import Heading from '@theme/Heading';
import styles from './index.module.css';
import React, { useEffect } from 'react';


export default function Home() {
  useEffect(() => {
    const script = document.createElement('script');
    script.src = '/js/homeLinkHighlight.js';
    script.async = true;
    document.body.appendChild(script);
    return () => {
      document.body.removeChild(script);
    };
  }, []);

  const { siteConfig } = useDocusaurusContext();

  return (
    <Layout
      title={`Documentation | ${siteConfig.title}`}
      description="Sequent Online Voting - End-to-end verifiable and transparent"
    >
      <HomepageHeader />
      <main>
        <HomepageFeatures />
      </main>
    </Layout>
  );
}


function HomepageHeader() {
  const { siteConfig } = useDocusaurusContext();
  return (
    <header className={clsx('hero hero--primary', styles.heroBanner)}>
      <div className="container">
        <Heading as="h1" className="hero__title">
          {siteConfig.title}
        </Heading>
        <p className="hero__subtitle">{siteConfig.tagline}</p>
        <div className={styles.buttons}>
          <Link
            className="button button--secondary button--lg"
            to="/docs/system_introduction/">
            System Introduction
          </Link>
        </div>
      </div>
    </header>
  );
}
