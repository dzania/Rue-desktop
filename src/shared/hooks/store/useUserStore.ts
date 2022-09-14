import create from "zustand";

import { UserStore } from "../../types";

const storeDefaultValues = {
	user: undefined,
};

export const useUserStore = create<UserStore>((set) => ({
	...storeDefaultValues,
	setState: (newState) => set((state) => ({ ...state, ...newState })),
	load: (user) =>
		set((state) => ({
			...state,
			user: user,
		})),
  setNone: () => set(() => storeDefaultValues),
	
}));
