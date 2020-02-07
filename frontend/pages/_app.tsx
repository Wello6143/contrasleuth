import "../styles/index.css";

const MyApp = ({ Component, pageProps }) => {
  return (
    <>
      <style jsx>{`
        .app-name {
          font-family: "Fira Sans";
          padding-top: 20px;
          padding-bottom: 20px;
          font-size: 30px;
          text-align: center;
        }
        .outer {
          padding-left: 10vw;
          padding-right: 10vw;
        }
      `}</style>
      <div className="app-name">Contrasleuth</div>
      <div className="outer">
        <Component {...pageProps} />
      </div>
    </>
  );
};

export default MyApp;
