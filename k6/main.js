import http from "k6/http";
import { check, sleep, group } from "k6";
import { randomItem } from "https://jslib.k6.io/k6-utils/1.2.0/index.js";
import { htmlReport } from "https://raw.githubusercontent.com/benc-uk/k6-reporter/main/dist/bundle.js";

export const options = {
  stages: [
    { duration: "30s", target: 20 },
    { duration: "1m", target: 20 },
    { duration: "30s", target: 0 },
  ],
  thresholds: {
    http_req_duration: ["p(95)<200"],
    http_req_failed: ["rate<0.01"],
    checks: ["rate==1.0"],
  },
};

const BASE_URL = __ENV.BASE_URL || "https://indexer.namada.tududes.com";
const ADDRESSES = [
  "tnam1q893l8zdx48wcdg62mf9gy4klgq9rmvg9ss48rz7",
  "tnam1q8w3wpa2rh9qepzyn72923lkd9aepwnsucw4wcp7",
  "tnam1qqec0hwnhqss75ngqgh4jrjxwgzxq6c8u5sr03mg",
  "tnam1qz9g9w3mnre6ravw3wrxw5mpzv4vtajy8s0rm8af",
];
const GOVERNANCE_PROPOSALS = [1, 10, 30];

export default function () {
  group("Health Check", function () {
    const res = http.get(`${BASE_URL}/health`);
    check(res, { "Health check is 200": (r) => r.status === 200 });
  });

  sleep(1);

  group("PoS Validator Endpoints", function () {
    const res = http.get(`${BASE_URL}/api/v1/pos/validator?page=1`, {
      tags: { name: "GetValidators" },
    });
    check(res, { "Get validators is 200": (r) => r.status === 200 });
  });

  sleep(1);

  group("PoS Rewards Endpoints", function () {
    const address = randomItem(ADDRESSES);
    const res = http.get(`${BASE_URL}/api/v1/pos/reward/${address}`, {
      tags: { name: "GetReward" },
    });
    check(res, { "Get reward is 200": (r) => r.status === 200 });
  });

  sleep(1);

  group("Governance proposal Endpoints", function () {
    const proposalId = randomItem(GOVERNANCE_PROPOSALS);
    const res = http.get(
      `${BASE_URL}/api/v1/gov/proposal/${proposalId}`,
      {
        tags: { name: "GetGovernanceProposal" },
      }
    );
    check(res, { "Get governance proposal is 200": (r) => r.status === 200 });
  });

  sleep(1);

  group("Governance votes Endpoints", function () {
    const proposalId = randomItem(GOVERNANCE_PROPOSALS);
    const res = http.get(
      `${BASE_URL}/api/v1/gov/proposal/${proposalId}/votes`,
      {
        tags: { name: "GetGovernanceVotes" },
      }
    );
    check(res, {
      "Get governance proposal votes is 200": (r) => r.status === 200,
    });
  });

  group("Account Balance Endpoints", function () {
    const address = randomItem(ADDRESSES);
    const res = http.get(`${BASE_URL}/api/v1/account/${address}`, {
      tags: { name: "GetAccountBalance" },
    });
    check(res, { "Get account balance is 200": (r) => r.status === 200 });
  });

  sleep(1);

  group("Masp Aggregated Endpoints", function () {
    const res = http.get(`${BASE_URL}/api/v1/masp/aggregates`, {
      tags: { name: "GetMaspAggregated" },
    });
    check(res, { "Get masp aggregated is 200": (r) => r.status === 200 });
  });
}

export function handleSummary(data) {
  return {
    "summary.html": htmlReport(data),
  };
}