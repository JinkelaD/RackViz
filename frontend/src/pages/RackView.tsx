import { useRackView } from '../hooks/useRackView';
import StatusBar from '../components/StatusBar';
import RoomTabs from '../components/RoomTabs';
import DeviceStockPanel from '../components/rack/DeviceStockPanel';
import RackCard from '../components/rack/RackCard';
import TypeLegend from '../components/rack/TypeLegend';
import AddRackModal from '../components/rack/AddRackModal';
import EditDeviceModal from '../components/rack/EditDeviceModal';
import EditRackModal from '../components/rack/EditRackModal';

export default function RackView() {
  const v = useRackView();

  return (
    <div className="rackview-wrapper">
      <div className="rackview-main">
        <div id="canvas-container">
          <TypeLegend />
          <div id="rack-grid" style={{ transform: `scale(${v.zoom / 100})` }}>
            {v.filteredRacks.length === 0 && (
              <div className="canvas-empty">
                <div className="canvas-empty-icon">⊞</div>
                <div className="canvas-empty-title">
                  {v.view === 'front' ? '暂无正面机柜' : '暂无背面机柜'}
                </div>
                <div className="canvas-empty-hint">点击底部「+ 机柜」添加新机柜</div>
              </div>
            )}
            {v.filteredRacks.map((rack, rackIdx) => {
              const stats = v.rackStats[rack.id] || { usedU: 0, deviceCount: 0 };
              return (
                <RackCard
                  key={rack.id}
                  rack={rack}
                  isFirst={rackIdx === 0}
                  isLast={rackIdx === v.filteredRacks.length - 1}
                  selected={v.selectedRackId === rack.id}
                  stats={stats}
                  dropTarget={v.dropTarget}
                  draggingDeviceId={v.draggingDevice?.id ?? null}
                  selectedDeviceId={v.selectedDevice?.id ?? null}
                  hoveredDeviceId={v.hoveredDeviceId}
                  hasSearch={v.searchQuery.trim().length > 0}
                  searchMatchedDeviceIds={v.searchMatchedDeviceIds}
                  devices={v.getDevicesForRack(rack.id)}
                  getDeviceModel={v.getDeviceModel}
                  getRoomName={v.getRoomName}
                  onRackClick={v.handleRackClick}
                  onRackDoubleClick={v.handleRackDoubleClick}
                  onMoveLeft={v.handleMoveRackLeft}
                  onMoveRight={v.handleMoveRackRight}
                  onDelete={v.handleRemoveRack}
                  onDragOver={v.handleDragOver}
                  onDragLeave={v.handleDragLeave}
                  onDrop={v.handleDrop}
                  onDeviceClick={v.handleDeviceClick}
                  onDeviceDoubleClick={v.handleDeviceDoubleClick}
                  onDeviceDragStart={v.handleDragStart}
                  onDeviceDragEnd={v.handleDragEnd}
                  onHoverDevice={v.setHoveredDeviceId}
                />
              );
            })}
          </div>
        </div>

        <aside id="sidebar">
          <DeviceStockPanel
            unassignedDevices={v.unassignedDevices}
            draggingDeviceId={v.draggingDevice?.id ?? null}
            selectedDeviceId={v.selectedDevice?.id ?? null}
            dragOverStock={v.dragOverStock}
            selectedDeviceInfo={v.selectedDeviceInfo}
            getDeviceModel={v.getDeviceModel}
            onDragStart={v.handleDragStart}
            onDragEnd={v.handleDragEnd}
            onStockDragOver={v.handleStockDragOver}
            onStockDrop={v.handleStockDrop}
            onDeviceClick={v.handleSidebarDeviceClick}
            onDeviceDoubleClick={v.handleDeviceDoubleClick}
            onCloseDetail={v.clearSelectedDevice}
            onUpdateDetail={v.update}
            onRemoveDetail={async (id) => { await v.remove(id); v.clearSelectedDevice(); }}
          />
        </aside>
      </div>

      <RoomTabs />

      <StatusBar
        zoom={v.zoom}
        onZoomIn={v.onZoomIn}
        onZoomOut={v.onZoomOut}
        onZoomReset={v.onZoomReset}
        searchQuery={v.searchQuery}
        onSearchChange={v.onSearchChange}
        onAddRack={v.addRack}
        onExportRackPlan={v.handleExportRackPlan}
        onExportSingleRack={v.handleExportSingleRack}
        singleRackExportDisabled={v.selectedRackId == null}
        deviceCount={v.displayStats.deviceCount}
        onlineCount={v.displayStats.onlineCount}
        totalUUsed={v.displayStats.totalUUsed}
        totalU={v.displayStats.totalU}
      />

      <AddRackModal
        open={v.showAddRack}
        rooms={v.rooms}
        defaultRoomId={v.selectedRoomId}
        onClose={() => v.setShowAddRack(false)}
        onCreate={v.handleAddRackSubmit}
      />

      <EditDeviceModal
        device={v.detailDevice}
        visible={v.deviceDetailVisible}
        models={v.models}
        racks={v.racks}
        onClose={v.closeDeviceDetail}
        onSave={v.handleDetailDeviceSave}
      />

      <EditRackModal
        rack={v.editingRack}
        visible={v.rackEditVisible}
        rooms={v.rooms}
        onClose={v.closeRackEdit}
        onSave={v.handleRackEditSave}
      />
    </div>
  );
}
