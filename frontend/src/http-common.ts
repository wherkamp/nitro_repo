import axios, {AxiosInstance} from 'axios';

const apiURL = (import.meta.env.VITE_API_URL === undefined) ? appURL : import.meta.env.VITE_API_URL;

const apiClient: AxiosInstance = axios.create({
  baseURL: apiURL,
  headers: {
    'Content-Type': 'application/json',
    Accept: 'application/json',
  },
  withCredentials: true,

});

export default {apiClient, apiURL};
