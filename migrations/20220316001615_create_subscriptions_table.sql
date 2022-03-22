-- Add migration script here
create table subscriptions(
  id uuid not null,
  primary key (id),
  email text not null unique,
  name text not null,
  subscribed_at timestamptz not null,
  status text not null
);
create table subscription_tokens(
  subscription_token text not null,
  subscriber_id uuid not null
    references subscriptions (id),
  primary key (subscription_token)
);
