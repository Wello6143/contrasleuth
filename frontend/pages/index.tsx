import Section from "../components/Section";
import Heading from "../components/Heading";
import Note from "../components/Note";
import Text from "../components/Text";
import TextField from "../components/TextField";
import Action from "../components/Action";

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
