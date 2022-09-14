export interface User {
	username: string;
	bridgeAddress: string;
}

export interface UserStore {
	user?: User;
  setState: (newState: Partial<UserStore>) => void;
  load: (user: User) => void;
  setNone: () => void;
}

export interface Bridge {
	internalIpAddress: string;	
}
