import { create } from "zustand";
import type { DownloadJob } from "../types";

interface DownloadsState {
  jobs: DownloadJob[];
  setJobs: (jobs: DownloadJob[]) => void;
  upsertJob: (job: DownloadJob) => void;
}

export const useDownloadsStore = create<DownloadsState>((set) => ({
  jobs: [],
  setJobs: (jobs) => set({ jobs }),
  upsertJob: (job) =>
    set((state) => {
      const idx = state.jobs.findIndex((j) => j.id === job.id);
      if (idx === -1) return { jobs: [job, ...state.jobs] };
      const jobs = [...state.jobs];
      jobs[idx] = job;
      return { jobs };
    }),
}));
