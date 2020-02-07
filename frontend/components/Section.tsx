const Section: React.FC = ({ children }) => (
  <>
    <style jsx>{`
      .section {
        border-left: 6px double black;
        padding-left: 6px;
        padding-top: 6px;
        margin-bottom: 6px;
        width: calc(100% - 6px - 6px);
      }
    `}</style>
    <div className="section">{children}</div>
  </>
);

export default Section;
