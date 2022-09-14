import { invoke } from "@tauri-apps/api";
import React, { useState } from "react";
import { Bridge } from "../../shared/types";

const SetupPage: React.FC = () => {
	const [bridges, setBridges] = useState<Bridge[] | string>();

	const createUser = () => {
		console.log();
	};

	invoke("mdns_discovery")
		.then((bridges) => {
			console.log(bridges);
			setBridges(bridges);
		})
		.catch((e) => setBridges(e));

	return (
		<> {bridges ? `Found ${bridges?.length} bridge` : "Looking for bridges"}</>
	);
};

export default SetupPage;
