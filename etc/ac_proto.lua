
local ac_proto = Proto("ac","Asheron's Call")

-- header fields
local pf_seq = ProtoField.uint32("ac.header.sequence", "Sequence")
local pf_flags = ProtoField.uint32("ac.header.flags", "Flags")
local pf_checksum = ProtoField.uint32("ac.header.checksum", "Checksum")
local pf_id = ProtoField.uint16("ac.header.id", "Id")
local pf_time = ProtoField.uint16("ac.header.time", "Time")
local pf_size = ProtoField.uint16("ac.header.size", "Size")
local pf_table = ProtoField.uint16("ac.header.table", "Table")
local pf_ack = ProtoField.uint32("ac.header.ack", "Ack")
local pf_resend = ProtoField.uint32("ac.header.resend", "Resend Count")
local pf_reject = ProtoField.uint32("ac.header.rejectResend", "Reject Count")
local pf_flow_bytes = ProtoField.uint32("ac.header.flowbytes", "Flow Bytes")
local pf_flow_time = ProtoField.uint32("ac.header.flowtime", "Flow Time")

local pf_data = ProtoField.bytes("ac.data", "Data")

-- local pf_frag_seq = ProtoField.uint32("ac.frag.sequence", "Sequence")
-- local pf_frag_id = ProtoField.uint32("ac.frag.id", "Id")
local pf_frag_seq = ProtoField.uint64("ac.frag.sequence", "Sequence", base.HEX)
local pf_frag_seq_eph = ProtoField.bool("ac.frag.sequence.eph", "UnOrdered", 32, nil, 0x80000000)
local pf_frag_seq_id = ProtoField.uint32("ac.frag.sequence.id", "Seq Id")
local pf_frag_seq_offset = ProtoField.uint32("ac.frag.sequence.offset", "Offset")

local pf_frag_count = ProtoField.uint16("ac.frag.count", "Count")
local pf_frag_size = ProtoField.uint16("ac.frag.size", "Size")
local pf_frag_index = ProtoField.uint16("ac.frag.index", "Index")
local pf_frag_group = ProtoField.uint16("ac.frag.group", "Group")
local pf_frag_data = ProtoField.bytes("ac.frag.data", "Data")
local pf_frag_type = ProtoField.uint32("ac.frag.type", "Type", base.HEX)
local pf_action_seq = ProtoField.uint32("ac.action.seq", "Action Sequence")
local pf_action_char = ProtoField.uint32("ac.action.char", "Character", base.HEX)
local pf_action_type = ProtoField.uint32("ac.action.type", "Type", base.HEX)

-- packet flags
local pf_flags_resent          = ProtoField.bool("ac.header.flags.resent", "Resent", 32, nil, 0x00000001)
local pf_flags_checksum        = ProtoField.bool("ac.header.flags.checksum", "Checksum", 32, nil, 0x00000002)
local pf_flags_fragment        = ProtoField.bool("ac.header.flags.fragment", "Fragment", 32, nil, 0x00000004)
local pf_flags_switch_server   = ProtoField.bool("ac.header.flags.switchServer", "Switch Server", 32, nil, 0x00000100)
local pf_flags_logon_server    = ProtoField.bool("ac.header.flags.logonServer", "Logon Server", 32, nil, 0x00000200)
local pf_flags_empty           = ProtoField.bool("ac.header.flags.empty", "Empty", 32, nil, 0x00000400)
local pf_flags_referral        = ProtoField.bool("ac.header.flags.referral", "Referral", 32, nil, 0x00000800)
local pf_flags_resend          = ProtoField.bool("ac.header.flags.resend", "Resend", 32, nil, 0x00001000)
local pf_flags_reject_resend   = ProtoField.bool("ac.header.flags.rejectResend", "Reject Resend", 32, nil, 0x00002000)
local pf_flags_ack             = ProtoField.bool("ac.header.flags.ack", "Ack", 32, nil, 0x00004000)
local pf_flags_disconnect      = ProtoField.bool("ac.header.flags.disconnect", "Disconnect", 32, nil, 0x00008000)
local pf_flags_logon           = ProtoField.bool("ac.header.flags.logon", "Logon", 32, nil, 0x00010000)
local pf_flags_accept          = ProtoField.bool("ac.header.flags.accept", "Accept", 32, nil, 0x00020000)
local pf_flags_connect_req     = ProtoField.bool("ac.header.flags.connectRequest", "Connect Request", 32, nil, 0x00040000)
local pf_flags_connect_res     = ProtoField.bool("ac.header.flags.connectResponse", "Connect Response", 32, nil, 0x00080000)
local pf_flags_net_error       = ProtoField.bool("ac.header.flags.netError", "Net Error", 32, nil, 0x00100000)
local pf_flags_net_disconnect  = ProtoField.bool("ac.header.flags.netDisconnect", "Net Disonnect", 32, nil, 0x00200000)
local pf_flags_command         = ProtoField.bool("ac.header.flags.command", "Command", 32, nil, 0x00400000)
local pf_flags_time_sync       = ProtoField.bool("ac.header.flags.timeSync", "Time Sync", 32, nil, 0x01000000)
local pf_flags_echo_req        = ProtoField.bool("ac.header.flags.echoRequest", "Echo Request", 32, nil, 0x02000000)
local pf_flags_echo_res        = ProtoField.bool("ac.header.flags.echoResponse", "Echo Response", 32, nil, 0x04000000)
local pf_flags_flow            = ProtoField.bool("ac.header.flags.flow", "Flow", 32, nil, 0x08000000)

ac_proto.fields = {
    pf_seq, pf_flags, pf_checksum, pf_id, pf_time, pf_size, pf_table,
    pf_ack, pf_resend, pf_reject, pf_flow_bytes, pf_flow_time, pf_data,
    pf_flags_resent, pf_flags_checksum, pf_flags_fragment,
    pf_flags_switch_server, pf_flags_logon_server, pf_flags_empty, pf_flags_referral,
    pf_flags_resend, pf_flags_reject_resend, pf_flags_ack, pf_flags_disconnect,
    pf_flags_logon, pf_flags_accept, pf_flags_connect_req, pf_flags_connect_res,
    pf_flags_net_error, pf_flags_net_disconnect, pf_flags_command, pf_flags_time_sync,
    pf_flags_echo_req, pf_flags_echo_res, pf_flags_flow,

    -- pf_frag_seq, pf_frag_id,
    pf_frag_seq, pf_frag_seq_eph, pf_frag_seq_id, pf_frag_seq_offset,
    pf_frag_count, pf_frag_size, pf_frag_index, pf_frag_group,
    pf_frag_data, pf_frag_type,

    pf_action_seq, pf_action_char, pf_action_type
}

local fv_flags_resend           = Field.new("ac.header.flags.resend")
local fv_flags_ack              = Field.new("ac.header.flags.ack")
local fv_flags_reject           = Field.new("ac.header.flags.rejectResend")
local fv_flags_sync             = Field.new("ac.header.flags.timeSync")
local fv_flags_echo_res         = Field.new("ac.header.flags.echoResponse")
local fv_flags_echo_req         = Field.new("ac.header.flags.echoRequest")
local fv_flags_flow             = Field.new("ac.header.flags.flow")
local fv_flags_fragment         = Field.new("ac.header.flags.fragment")

-- create a function to dissect it

function ac_proto.dissector(buffer,pinfo,tree)
    pinfo.cols.protocol = "AC"
    local info = ""
    local root = tree:add(ac_proto,buffer())
    local header = root:add(buffer(0,20), "Header")

    header:add_le(pf_seq, buffer(0,4))
    --info = info .. string.format("SEQ#: %d ", buffer(0, 4):le_uint())

    local flags = header:add_le(pf_flags, buffer(4,4))
    add_packet_flags(buffer(4,4), flags)
    header:add_le(pf_checksum, buffer(8,4))
    header:add_le(pf_id, buffer(12,2))
    header:add_le(pf_time, buffer(14,2))
    header:add_le(pf_size, buffer(16,2))
    header:add_le(pf_table, buffer(18,2))

    local position = 20
    local size = buffer(16,2):le_uint()

    if fv_flags_resend()() then
        local count = buffer(position, 4):le_uint()
        header:add_le(pf_resend, buffer(position, 4))
        position = position + 4 * (count + 1)
    end

    if fv_flags_reject()() then
        local count = buffer(position, 4):le_uint()
        header:add_le(pf_reject, buffer(position, 4))
        position = position + 4 * (count + 1)
    end

    if fv_flags_ack()() then
        header:add_le(pf_ack, buffer(position, 4))
        position = position + 4
    end

    if fv_flags_sync()() then
        local sync = header:add(ac_proto, buffer(position, 8), "Sync")
        position = position + 8
    end

    if fv_flags_echo_req()() then
        local echo = header:add(ac_proto, buffer(position, 4), "Echo Req")
        position = position + 4
    end

    if fv_flags_echo_res()() then
        local echo = header:add(ac_proto, buffer(position, 8), "Echo Res")
        position = position + 8
    end

    if fv_flags_flow()() then
        -- local flow = header:add(ac_proto, buffer(position, 6), "Flow")
        header:add_le(pf_flow_bytes, buffer(position, 4))
        header:add_le(pf_flow_time, buffer(position + 4, 2))
        position = position + 6
    end

    if fv_flags_fragment()() then
        while position < size + 20 do
            local fragment = root:add("Fragment")
            local seq = fragment:add_le(pf_frag_seq, buffer(position, 8))
            seq:add_le(pf_frag_seq_eph, buffer(position + 4, 4))
            seq:add_le(pf_frag_seq_id, buffer(position + 4, 2))
            seq:add_le(pf_frag_seq_offset, buffer(position, 4))

            -- fragment:add_le(pf_frag_id, buffer(position + 4, 4))
            fragment:add_le(pf_frag_count, buffer(position + 8, 2))
            fragment:add_le(pf_frag_size, buffer(position + 10, 2))
            fragment:add_le(pf_frag_index, buffer(position + 12, 2))
            fragment:add_le(pf_frag_group, buffer(position + 14, 2))

            local fragIndex = buffer(position + 12, 2):le_uint()
            if fragIndex == 0 then
                fragment:add_le(pf_frag_type, buffer(position + 16, 4))
                local fragType = buffer(position + 16, 4):le_uint()

                info = info .. string.format("%04X", fragType)

                if fragType == 0xf7b0 or fragType == 0xf7b1 then
                    local action = fragment:add("Action")
                    local actionPos = position + 16
                    if fragType == 0xf7b0 then
                        action:add_le(pf_action_char, buffer(actionPos, 4))
                        actionPos = actionPos + 4
                    end
                    action:add_le(pf_action_seq, buffer(actionPos + 4, 4))
                    action:add_le(pf_action_type, buffer(actionPos + 8, 4))
                    local actionType = buffer(actionPos + 8, 4):le_uint()
                    info = info .. string.format("/%04X", actionType)
                end
            else
                info = info .. "Frag"
            end
            
            local fragSize = buffer(position + 10, 2):le_uint()
            fragment:add_le(pf_frag_data, buffer(position + 16, fragSize - 16))
            position = position + fragSize

            info = info .. " "
        end

        pinfo.cols.info = info
    else
        pinfo.cols.info = info .. "Header Only"
        local data = root:add(ac_proto,buffer(20,size),"Data")
        data:add(pf_data, buffer(20,size))
    end
end

add_packet_flags = function(buffer,tree)
    tree:add_le(pf_flags_resent, buffer)
    tree:add_le(pf_flags_checksum, buffer)
    tree:add_le(pf_flags_fragment, buffer)
    tree:add_le(pf_flags_switch_server, buffer)
    tree:add_le(pf_flags_logon_server, buffer)
    tree:add_le(pf_flags_empty, buffer)
    tree:add_le(pf_flags_referral, buffer)
    tree:add_le(pf_flags_resend, buffer)
    tree:add_le(pf_flags_reject_resend, buffer)
    tree:add_le(pf_flags_ack, buffer)
    tree:add_le(pf_flags_disconnect, buffer)
    tree:add_le(pf_flags_logon, buffer)
    tree:add_le(pf_flags_accept, buffer)
    tree:add_le(pf_flags_connect_req, buffer)
    tree:add_le(pf_flags_connect_res, buffer)
    tree:add_le(pf_flags_net_error, buffer)
    tree:add_le(pf_flags_net_disconnect, buffer)
    tree:add_le(pf_flags_command, buffer)
    tree:add_le(pf_flags_time_sync, buffer)
    tree:add_le(pf_flags_echo_req, buffer)
    tree:add_le(pf_flags_echo_res, buffer)
    tree:add_le(pf_flags_flow, buffer)
end

-- load the udp.port table
local udp_table = DissectorTable.get("udp.port")
-- register our protocol to handle udp port 7777
udp_table:add("9000-9051",ac_proto)
-- udp_table:add(9001,ac_proto)

