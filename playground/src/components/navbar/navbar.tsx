import React from 'react';
import Logo from '../../assets/lox.png';

interface NavBarProps {
  /**
   * Set to `true` if VM is currently running.
   */
  isRunning: boolean;
  onRunClick: () => void;
};

/**
 * Navbar component
 */
const Navbar = ({ onRunClick, isRunning }: NavBarProps) => {
  let runColor = 'btn-success';
  let runIcon = 'me-1 bi bi-play-fill';
  let runText = 'Run';

  if (isRunning) {
    runColor = 'btn-danger';
    runIcon = 'me-2 spinner-grow spinner-grow-sm';
    runText = 'Stop';
  }

  return (
    <nav className="navbar p-2" id="navbar">
      <div className="navbar-brand">
        <img alt="Logo" className="me-2" src={Logo} />
        Loxcraft Playground
      </div>
      <div>
        <button
          className="btn btn-dark bi bi-github me-1"
          type="button"
          onClick={() => { window.open('https://github.com/ajeetdsouza/loxcraft', '_blank'); }}
          aria-label="Github repository"
        />
        <button id="run-btn" className={`btn ${runColor}`} onClick={onRunClick} type="button" aria-label="Run code">
          <span className={runIcon} role="status" aria-hidden="true" />
          {runText}
        </button>
      </div>
    </nav>
  );
}

export { Navbar, NavBarProps };
