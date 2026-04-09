import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import * as portfolioApi from "../api/portfolios";

export function usePortfolios() {
  return useQuery({
    queryKey: ["portfolios"],
    queryFn: portfolioApi.listPortfolios,
  });
}

export function usePortfolio(id: number) {
  return useQuery({
    queryKey: ["portfolio", id],
    queryFn: () => portfolioApi.getPortfolio(id),
  });
}

export function useHoldings(portfolioId: number) {
  return useQuery({
    queryKey: ["holdings", portfolioId],
    queryFn: () => portfolioApi.getHoldings(portfolioId),
  });
}

export function useCash(portfolioId: number) {
  return useQuery({
    queryKey: ["cash", portfolioId],
    queryFn: () => portfolioApi.getCash(portfolioId),
  });
}

export function useSnapshots(portfolioId: number) {
  return useQuery({
    queryKey: ["snapshots", portfolioId],
    queryFn: () => portfolioApi.getSnapshots(portfolioId),
  });
}

export function useTotalReturn(portfolioId: number) {
  return useQuery({
    queryKey: ["totalReturn", portfolioId],
    queryFn: () => portfolioApi.getTotalReturn(portfolioId),
  });
}

export function useCreatePortfolio() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => portfolioApi.createPortfolio(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["portfolios"] });
    },
  });
}
