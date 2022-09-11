import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";
import { Router, Route, Routes } from "react-router-dom";
import SetupPage from './pages/SetupPage';

function App() {

	const user = async () => {
		await invoke("load");
	};

	return (
		<div id="app">
			<Router>
				<Routes>
					<Route path="*" element={<SetupPage />} />
				</Routes>
			</Router>
		</div>
	);
}

export default App;
