const TextField: React.FC<{
  onChange?: (value: string) => void;
  value?: string;
}> = ({ onChange, value }) => (
  <>
    <style jsx>{`
      .text-field {
        line-height: inherit;
        font-size: inherit;
        width: 100%;
        margin-left: 7px;
        padding-left: 3px;
        padding-right: 3px;
        font-family: inherit;
        outline: 1px solid rgba(0, 0, 0, 0.6);
      }
      .text-field:focus {
        outline: 2px solid black;
      }
    `}</style>
    <input
      className="text-field"
      onChange={event => onChange(event.target.value)}
      value={value}
    />
  </>
);

export default TextField;
