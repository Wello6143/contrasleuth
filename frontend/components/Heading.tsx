const Heading: React.FC = ({ children }) => (
  <>
    <style>{`
      .heading {
        font-weight: 500;
        padding-bottom: 7px;
        font-size: 18px;
        line-height: 23px;
      }
    `}</style>
    <h1 className="heading">{children}</h1>
  </>
);

export default Heading;
