export interface GroupRow {
  id: number;
  name: string;
  description: string | null;
  subscribed: boolean;
  article_count: number;
  first_article: number;
  last_article: number;
  last_scanned: number;
  last_updated: string | null;
  created_at: string;
  unread_count: number;
}

export interface GroupListResponse {
  groups: GroupRow[];
  total: number;
  limit: number;
  offset: number;
}

export interface HeaderRow {
  id: number;
  group_id: number;
  article_num: number;
  subject: string;
  author: string;
  date: string;
  message_id: string;
  references_: string;
  bytes: number;
  lines: number;
  read: boolean;
  downloaded_at: string;
}

export interface HeaderListResponse {
  headers: HeaderRow[];
  total: number;
  limit: number;
  offset: number;
}

export interface ThreadSummary {
  root_message_id: string;
  subject: string;
  author: string;
  date: string;
  last_reply_date: string;
  reply_count: number;
  unread_count: number;
}

export interface ThreadArticle {
  id: number;
  group_id: number;
  article_num: number;
  subject: string;
  author: string;
  date: string;
  message_id: string;
  references_: string;
  bytes: number;
  lines: number;
  read: boolean;
  downloaded_at: string;
  depth: number;
}

export interface GroupStatusResponse {
  group_id: number;
  name: string;
  last_scanned: number;
  last_article: number;
  new_available: number;
  total_headers: number;
  unread_count: number;
  last_updated: string | null;
}
