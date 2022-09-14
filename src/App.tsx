import { invoke } from "@tauri-apps/api/tauri";
import shallow from "zustand/shallow";
import {
	BrowserRouter as Router,
	Route,
	Routes,
	Navigate,
} from "react-router-dom";
import { useUserStore } from "./shared/hooks/store/useUserStore";
import SetupPage from "./pages/SetupPage/SetupPage";
import { useEffect } from "react";

function App() {
	const [user, load, setNone] = useUserStore(
		(state) => [state.user, state.load, state.setNone],
		shallow
	);

	useEffect(() => {
		invoke("load")
			.then((user) => {
				load(user);
			})
			.catch(() => setNone());
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, []);

	return (
		<div id="app">
			<Router>
				<Routes>
					<Route path="/setup" element={<SetupPage />} />
					<Route
						path="*"
						element={
							user ? (
								<Navigate replace to="/lights" />
							) : (
								<Navigate replace to="/setup" />
							)
						}
					/>
				</Routes>
			</Router>
		</div>
	);
>>>>>>> Stashed changes
}

export default App;
