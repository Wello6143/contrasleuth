const Action: React.FC<{ onClick?: () => void }> = ({ children, onClick }) => (
  <>
    <style jsx>{`
      .action {
        color: inherit;
      }
    `}</style>
    <a className="action" href="javascript:void 0" onClick={onClick}>
      {children}
    </a>
  </>
);

export default Action;
