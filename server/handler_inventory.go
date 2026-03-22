// 008 P5：實體狀態、背包、裝備與 pushRefresh。
package server

import (
	"encoding/json"

	"singularity_world/db"
	"singularity_world/gametext"
)

func handleGetEntityStatus(c *Client, msg *ClientMsg) {
	if c.PlayerID == "" {
		sendError(c, gametext.Client("need_login"))
		return
	}
	entityID := msg.EntityID
	if entityID == "" {
		entityID = c.PlayerID
	}
	ent, err := db.GetEntity(entityID)
	if err != nil || ent == nil {
		sendError(c, gametext.Client("status_entity_not_found"))
		return
	}
	isSelf := entityID == c.PlayerID
	mag := ent.Magnesium
	var magPtr *int
	if isSelf {
		magPtr = &mag
	}
	rm := db.ComputeResourceMaxes(ent.Vit, ent.Qi, ent.Dex)
	status := EntityStatusMsg{
		Type:        "entity_status",
		EntityID:    ent.ID,
		DisplayChar: ent.DisplayChar,
		Vit:         ent.Vit,
		Qi:          ent.Qi,
		Dex:         ent.Dex,
		HpCur:       int(rm.HpCur), HpMax: int(rm.HpMax),
		InnerCur: int(rm.InnerCur), InnerMax: int(rm.InnerMax),
		SpiritCur: int(rm.SpiritCur), SpiritMax: int(rm.SpiritMax),
		StaminaCur: int(rm.StaminaCur), StaminaMax: int(rm.StaminaMax),
		Magnesium: magPtr,
		IsSelf:    isSelf,
	}
	if ent.DisplayTitle != "" {
		status.DisplayTitle = ent.DisplayTitle
	}
	if isSelf {
		status.ActivatedNodes = parseActivatedNodes(ent.ActivatedNodes)
		if ent.SoulSeed != nil {
			status.OriginSentence = db.ExpandSoulSeedToOriginSentence(*ent.SoulSeed)
			status.TopologyCosts = db.ExpandSoulSeedToTopologyCosts(*ent.SoulSeed)
		}
	}
	status.EquipmentSlots, status.EquipmentNames, status.EquipmentDescs = parseEquipment(ent.EquipmentSlots)
	c.Send <- mustJSON(status)
}

func handleGetInventory(c *Client) {
	if c.PlayerID == "" {
		sendError(c, gametext.Client("need_login"))
		return
	}
	ent, err := db.GetEntity(c.PlayerID)
	if err != nil || ent == nil {
		sendError(c, gametext.Client("inv_entity_not_found"))
		return
	}
	result := db.GetInventory(ent.Inventory, ent.Vit)
	items := make([]InventoryItemView, 0, len(result.Items))
	for _, it := range result.Items {
		items = append(items, InventoryItemView{
			ItemID:      it.ItemID,
			Name:        it.Name,
			ItemType:    it.ItemType,
			Qty:         it.Qty,
			Weight:      it.Weight,
			SubTotal:    it.SubTotal,
			Description: it.Description,
			Slot:        it.Slot,
		})
	}
	c.Send <- mustJSON(InventoryMsg{
		Type:          "inventory",
		Items:         items,
		CurrentWeight: result.CurrentWeight,
		MaxWeight:     result.MaxWeight,
	})
}

func handleEquipItem(c *Client, msg *ClientMsg) {
	if c.PlayerID == "" {
		sendError(c, gametext.Client("need_login"))
		return
	}
	itemID := msg.ItemID
	if itemID == "" {
		sendError(c, gametext.Client("inv_no_item"))
		return
	}
	_, itemType, slot, _, _, err := db.GetItemInfo(itemID)
	if err != nil {
		sendError(c, gametext.Client("inv_item_not_found"))
		return
	}
	if itemType != "equipment" || slot == "" {
		sendError(c, gametext.Client("inv_cannot_equip"))
		return
	}
	targetSlot := slot
	if slot == "hold" {
		targetSlot = msg.TargetSlot
		if targetSlot != "hold_l" && targetSlot != "hold_r" {
			sendError(c, gametext.Client("inv_hand_slot"))
			return
		}
	}

	ent, err := db.GetEntity(c.PlayerID)
	if err != nil || ent == nil {
		sendError(c, gametext.Client("inv_entity_not_found"))
		return
	}

	var invEntries []db.InventoryEntry
	if ent.Inventory != "" && ent.Inventory != "[]" {
		_ = json.Unmarshal([]byte(ent.Inventory), &invEntries)
	}
	hasItem := false
	for _, e := range invEntries {
		if e.ItemID == itemID && e.Qty > 0 {
			hasItem = true
			break
		}
	}
	if !hasItem {
		sendError(c, gametext.Client("inv_bag_missing"))
		return
	}

	var currentSlots map[string]string
	if ent.EquipmentSlots != "" {
		_ = json.Unmarshal([]byte(ent.EquipmentSlots), &currentSlots)
	}
	if currentSlots == nil {
		currentSlots = make(map[string]string)
	}
	oldItemID := currentSlots[targetSlot]

	if err := db.RemoveFromInventory(c.PlayerID, itemID, 1); err != nil {
		sendError(c, gametext.Client("inv_bag_fail"))
		return
	}
	if err := db.UpdateEquipmentSlot(c.PlayerID, targetSlot, itemID); err != nil {
		sendError(c, gametext.Client("inv_equip_fail"))
		return
	}
	if oldItemID != "" {
		if err := db.AddToInventory(c.PlayerID, oldItemID, 1); err != nil {
			sendError(c, gametext.Client("inv_unequip_old_fail"))
			return
		}
	}

	pushRefresh(c)
}

func handleUnequipItem(c *Client, msg *ClientMsg) {
	if c.PlayerID == "" {
		sendError(c, gametext.Client("need_login"))
		return
	}
	slotCode := msg.Slot
	if slotCode == "" {
		sendError(c, gametext.Client("unequip_no_slot"))
		return
	}

	ent, err := db.GetEntity(c.PlayerID)
	if err != nil || ent == nil {
		sendError(c, gametext.Client("inv_entity_not_found"))
		return
	}

	var currentSlots map[string]string
	if ent.EquipmentSlots != "" {
		_ = json.Unmarshal([]byte(ent.EquipmentSlots), &currentSlots)
	}
	if currentSlots == nil {
		sendError(c, gametext.Client("unequip_nothing"))
		return
	}
	itemID := currentSlots[slotCode]
	if itemID == "" {
		sendError(c, gametext.Client("unequip_slot_empty"))
		return
	}

	_, _, _, _, weight, wErr := db.GetItemInfo(itemID)
	if wErr != nil {
		weight = 0
	}
	currentWeight := db.InventoryWeight(ent.Inventory)
	maxWeight := float64(ent.Vit) * 10.0
	if currentWeight+weight > maxWeight {
		sendError(c, gametext.Client("unequip_bag_full"))
		return
	}

	if err := db.ClearEquipmentSlot(c.PlayerID, slotCode); err != nil {
		sendError(c, gametext.Client("unequip_slot_fail"))
		return
	}
	if err := db.AddToInventory(c.PlayerID, itemID, 1); err != nil {
		sendError(c, gametext.Client("inv_bag_fail"))
		return
	}

	pushRefresh(c)
}

func pushRefresh(c *Client) {
	ent, err := db.GetEntity(c.PlayerID)
	if err != nil || ent == nil {
		return
	}
	rm := db.ComputeResourceMaxes(ent.Vit, ent.Qi, ent.Dex)
	invResult := db.GetInventory(ent.Inventory, ent.Vit)
	items := make([]InventoryItemView, 0, len(invResult.Items))
	for _, it := range invResult.Items {
		items = append(items, InventoryItemView{
			ItemID:      it.ItemID,
			Name:        it.Name,
			ItemType:    it.ItemType,
			Qty:         it.Qty,
			Weight:      it.Weight,
			SubTotal:    it.SubTotal,
			Description: it.Description,
			Slot:        it.Slot,
		})
	}
	c.Send <- mustJSON(InventoryMsg{
		Type:          "inventory",
		Items:         items,
		CurrentWeight: invResult.CurrentWeight,
		MaxWeight:     invResult.MaxWeight,
	})

	isSelf := true
	mag := ent.Magnesium
	status := EntityStatusMsg{
		Type:        "entity_status",
		EntityID:    ent.ID,
		DisplayChar: ent.DisplayChar,
		Vit:         ent.Vit, Qi: ent.Qi, Dex: ent.Dex,
		HpCur: int(rm.HpCur), HpMax: int(rm.HpMax),
		InnerCur: int(rm.InnerCur), InnerMax: int(rm.InnerMax),
		SpiritCur: int(rm.SpiritCur), SpiritMax: int(rm.SpiritMax),
		StaminaCur: int(rm.StaminaCur), StaminaMax: int(rm.StaminaMax),
		Magnesium: &mag,
		IsSelf:    isSelf,
	}
	if ent.DisplayTitle != "" {
		status.DisplayTitle = ent.DisplayTitle
	}
	status.ActivatedNodes = parseActivatedNodes(ent.ActivatedNodes)
	if ent.SoulSeed != nil {
		status.OriginSentence = db.ExpandSoulSeedToOriginSentence(*ent.SoulSeed)
		status.TopologyCosts = db.ExpandSoulSeedToTopologyCosts(*ent.SoulSeed)
	}
	status.EquipmentSlots, status.EquipmentNames, status.EquipmentDescs = parseEquipment(ent.EquipmentSlots)
	c.Send <- mustJSON(status)
}
