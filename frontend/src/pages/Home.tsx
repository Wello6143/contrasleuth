import React from "react";
import Section from "../theme/Section";
import Heading from "../theme/Heading";
import Note from "../theme/Note";
import Text from "../theme/Text";
import TextField from "../theme/TextField";
import Action from "../theme/Action";

const Home = () => (
  <>
    <Heading>Inboxes</Heading>
    <Text>
      Nothing here yet. Do you want to <Action>create a new inbox?</Action>
    </Text>
    <Section>
      <Heading>New inbox</Heading>
      <Text>
        <div style={{ display: "flex" }}>
          <span style={{ lineHeight: "30px" }}>Label:</span>
          <TextField />
        </div>
      </Text>
      <Text>
        Address <Note>(randomly assigned)</Note>: f23ieifabwsdtu7x
      </Text>
    </Section>
  </>
);

export default Home;
