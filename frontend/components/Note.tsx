const Note: React.FC = ({ children }) => (
  <>
    <style>{`
      .note {
        font-style: italic;
      }
    `}</style>
    <em className="note">{children}</em>
  </>
);

export default Note;
