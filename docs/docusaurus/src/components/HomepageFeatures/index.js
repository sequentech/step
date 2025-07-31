import clsx from 'clsx';
import Heading from '@theme/Heading';
import styles from './styles.module.css';
import Link from '@docusaurus/Link';

import ElectionSvg from '@site/static/img/election_managers.svg';
import VotersSvg from '@site/static/img/voters.svg';
import AuditorsSvg from '@site/static/img/auditors.svg';

function Feature({Svg, title, description, link}) {
  return (
    <div className="col col--4">
      <a href={link} style={{ textDecoration: 'none', color: 'inherit' }}>
        <div className="text--center">
          {Svg ? (
            <Svg className="featureSvg" role="img" />
          ) : (
            <img src={link} alt={title} className="featureSvg" />
          )}
        </div>
        <div className="text--center padding-horiz--md">
          <h3>{title}</h3>
          <p>{description}</p>
        </div>
      </a>
    </div>
  );
}

const FeatureList = [
  {
    title: 'Election Creation - test 1',
    Svg: ElectionSvg,
    link: '/docs/admin_portal/Tutorials/admin_portal_tutorials_setting-up-your-first-election',
    description: (
      <>
        Learn how election managers configure and launch elections,
        from creating contests and candidates to publishing results.
      </>
    ),
  },
  {
    title: 'Voting Process - test 2',
    Svg: VotersSvg,
    link: 'docs/voting_portal/voting_portal.md',
    description: (
      <>
        Explore the complete voting experience from the voter's perspective,
        including ballot navigation, submission steps, and verification tools.
      </>
    ),
  },
  {
    title: 'Election Audit - test 3',
    Svg: AuditorsSvg,
    link: 'docs/election_verifier/election_verifier.md',
    description: (
      <>
        Understand how auditors - whether public observers or officials use tools
        such as the Ballot Verifier, Ballot Auditor, and Election Verifier to ensure election transparency and integrity.
      </>
    ),
  },
];

export default function HomepageFeatures() {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
