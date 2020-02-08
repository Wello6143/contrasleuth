import React from "react";
import { Redirect, Route } from "react-router-dom";
import { IonApp, IonRouterOutlet } from "@ionic/react";
import { IonReactRouter } from "@ionic/react-router";
import styled from "styled-components";
import Home from "./pages/Home";
import "./index.css";

const AppName = styled.div`
  padding-top: 20px;
  padding-bottom: 20px;
  font-size: 30px;
  text-align: center;
`;

const Outer = styled.div`
  padding-left: 10vw;
  padding-right: 10vw;
`;

const App: React.FC = () => (
  <IonApp>
    <IonReactRouter>
      <IonRouterOutlet>
        <Outer>
          <AppName>Contrasleuth</AppName>
          <Route path="/home" component={Home} exact={true} />
          <Route exact path="/" render={() => <Redirect to="/home" />} />
        </Outer>
      </IonRouterOutlet>
    </IonReactRouter>
  </IonApp>
);

export default App;
