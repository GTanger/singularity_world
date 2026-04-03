-- 清除除玩家「葉卅」外之實體與 NPC 關聯資料（PostgreSQL）。
-- 使用：psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f scripts/purge_npcs_keep_player_葉卅.sql
-- 若 DATABASE_URL 未設，可：psql postgres://postgres:singularity@localhost:5432/singularity -f ...

BEGIN;

DELETE FROM assignments;
DELETE FROM schedules;

DELETE FROM entity_rooms WHERE entity_id <> '葉卅';
DELETE FROM entities WHERE id <> '葉卅';

DELETE FROM archival WHERE entity_id <> '葉卅';
DELETE FROM event_log WHERE entity_id <> '葉卅';
DELETE FROM auth WHERE entity_id <> '葉卅';

DELETE FROM npc_memories;
DELETE FROM npc_summaries;
DELETE FROM npc_npc_summaries;
DELETE FROM npc_threads;
DELETE FROM npc_dyads;
DELETE FROM npc_rumors;

COMMIT;
