function highlightHomeLink() {
  const homeLink = [...document.querySelectorAll('.navbar__title')].find(el => el.textContent.trim() === 'Home');
  if (homeLink) {
    homeLink.style.color = '#2EE8B9'; // Mint green
    homeLink.style.backgroundColor = 'transparent';
    homeLink.style.borderRadius = '0';
    homeLink.style.paddingLeft = '15px';
  } else {
    console.warn('Home link not found');
  }
}

highlightHomeLink();