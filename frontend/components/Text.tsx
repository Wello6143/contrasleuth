const Text: React.FC = ({ children }) => (
  <>
    <style jsx>{`
      .text {
        padding-bottom: 7px;
        font-size: 17px;
        line-height: 23px;
      }
    `}</style>
    <div className="text">{children}</div>
  </>
);

export default Text;
